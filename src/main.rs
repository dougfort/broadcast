use anyhow::Error;
use crdts::{CmRDT, CvRDT, Map, Orswot};
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, watch};

mod names;
mod signal;

type ActorId = usize;
type FriendMap = Map<String, Orswot<String, ActorId>, ActorId>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Set the RUST_LOG, if it hasn't been explicitly defined
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "broadcast=debug")
    }
    tracing_subscriber::fmt::init();

    // TODO: #1 load config from environment and/or command line

    const ACTOR_COUNT: usize = 10;
    const BROADCAST_CHANNEL_SIZE: usize = ACTOR_COUNT;
    const NAMES_PATH: &str = "data/names.txt";

    tracing::info!("program starts");

    let (broadcast_tx, _) = broadcast::channel::<(usize, String)>(BROADCAST_CHANNEL_SIZE);
    let broadcast_tx = Arc::new(broadcast_tx);
    let names = Arc::new(names::Names::new(NAMES_PATH)?);

    let mut halt = false;
    let (halt_tx, halt_rx) = watch::channel(halt);

    let mut join_handles = vec![];
    for actor_id in 1..=ACTOR_COUNT {
        let names = names.clone();
        let broadcast_tx = broadcast_tx.clone();
        let broadcast_rx = broadcast_tx.subscribe();
        let halt_rx = halt_rx.clone();
        let join_handle = tokio::spawn(async move {
            actor(actor_id, names, broadcast_tx, broadcast_rx, halt_rx).await?;

            Ok::<(), Error>(())
        });
        join_handles.push((actor_id, join_handle));
    }

    tracing::info!("waiting for shutdown signal");
    signal::shutdown_signal().await;

    halt = true;
    halt_tx.send(halt)?;

    for (actor_id, join_handle) in join_handles {
        let result = join_handle.await?;
        tracing::debug!("actor: {:03}; join result = {:?}", actor_id, result);
    }

    tracing::info!("progam terminates normally");
    Ok(())
}

async fn actor(
    actor_id: usize,
    names: Arc<names::Names>,
    broadcast_tx: Arc<broadcast::Sender<(usize, String)>>,
    mut broadcast_rx: broadcast::Receiver<(usize, String)>,
    mut halt_rx: watch::Receiver<bool>,
) -> Result<(), Error> {
    let mut halt = *halt_rx.borrow();
    let mut friend_map: FriendMap = Map::new();
    let mut trans_count: usize = 0;
    let mut after_count: usize = 0;

    while !halt {
        let sleep_interval = {
            let mut rng = rand::thread_rng();
            Duration::from_secs(rng.gen_range(0..20))
        };
        tokio::select! {
            _ = tokio::time::sleep(sleep_interval) => {
                let op = mutate_map(actor_id, names.clone(), &friend_map)?;
                friend_map.apply(op);
                let map_string = serde_json::to_string(&friend_map)?;
                broadcast_tx.send((actor_id, map_string))?;
            }
            _ = halt_rx.changed() => {halt = *halt_rx.borrow()}
            recv_result = broadcast_rx.recv() => {
                match recv_result {
                    Ok((incoming_actor_id, incoming_data)) => {
                        if incoming_actor_id != actor_id {
                            trans_count += 1;
                            let incoming_map: FriendMap = serde_json::from_str(&incoming_data)?;
                            let test_map = incoming_map.clone();
                            friend_map.validate_merge(&incoming_map)?;
                            friend_map.merge(incoming_map);
                            if test_map == friend_map {
                                after_count += 1;
                            }
                            let percentage: f64 = (after_count as f64 / trans_count as f64) * 100.0;
                            tracing::debug!(
                                "Actor({:03}) from Actor({:03}) {:>5} of {:>5} {:.2}%",
                                actor_id, incoming_actor_id, after_count, trans_count, percentage,
                            );
                        }
                    }
                    Err(error) => match error {
                        broadcast::error::RecvError::Closed => {
                            tracing::info!("actor: {:03}; connection closed", actor_id);
                            halt = true;
                        },
                        broadcast::error::RecvError::Lagged(lag_count) => {
                            tracing::warn!("actor: {:03}; lagged by {}", actor_id, lag_count);
                        }
                    }
                }
            }
        }
    }

    tracing::debug!("actor: {} terminates", actor_id);
    Ok(())
}

/// make a change to the map to be broadcast to other Actors
/// if the size of the map is less that minimum, add a key
/// if the size of the map is greater than maximum, delete a key
/// otherwise add a value to a random key
fn mutate_map(
    actor_id: usize,
    names: Arc<names::Names>,
    friend_map: &FriendMap,
) -> Result<crdts::map::Op<String, Orswot<String, usize>, usize>, Error> {
    const MIN_SIZE: usize = 10;
    const MAX_SIZE: usize = 20;

    let read_ctx = friend_map.len();
    let len: usize = read_ctx.val;

    let op = if len < MIN_SIZE {
        let key = names.choose()?;
        let value = names.choose()?;
        tracing::debug!("Actor {:03}: adding key {} with {}", actor_id, key, value);
        friend_map.update(
            key.as_str(),
            read_ctx.derive_add_ctx(actor_id),
            |set, ctx| set.add(value, ctx),
        )
    } else if len > MAX_SIZE {
        let mut rng = thread_rng();
        let keys: Vec<&String> = friend_map.keys().map(|c| c.val).collect();
        let key = keys.choose(&mut rng).unwrap();
        tracing::debug!("Actor {:03}: removing key {}", actor_id, key);
        friend_map.rm(*key, read_ctx.derive_rm_ctx())
    } else {
        let mut rng = thread_rng();
        let keys: Vec<&String> = friend_map.keys().map(|c| c.val).collect();
        let key = keys.choose(&mut rng).unwrap();
        let value = names.choose()?;
        tracing::debug!(
            "Actor {:03}: adding value {} to key {}",
            actor_id,
            value,
            key
        );
        friend_map.update(
            key.as_str(),
            read_ctx.derive_add_ctx(actor_id),
            |set, ctx| set.add(value, ctx),
        )
    };

    Ok(op)
}
