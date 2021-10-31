use anyhow::Error;
use crdts::{CmRDT, CvRDT, Map, Orswot};
use rand::Rng;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, watch};

mod names;
mod signal;

type FriendMap = Map<String, Orswot<String, usize>, usize>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Set the RUST_LOG, if it hasn't been explicitly defined
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "broadcast=debug")
    }
    tracing_subscriber::fmt::init();

    // TODO: #1 load config from environment and/or command line

    const ACTOR_COUNT: usize = 2;
    const BROADCAST_CHANNEL_SIZE: usize = ACTOR_COUNT;
    const NAMES_PATH: &str = "data/names.txt";

    tracing::info!("program starts");

    let (broadcast_tx, _) = broadcast::channel::<String>(BROADCAST_CHANNEL_SIZE);
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
    broadcast_tx: Arc<broadcast::Sender<String>>,
    mut broadcast_rx: broadcast::Receiver<String>,
    mut halt_rx: watch::Receiver<bool>,
) -> Result<(), Error> {
    let mut halt = *halt_rx.borrow();
    let mut friend_map: FriendMap = Map::new();

    while !halt {
        let sleep_interval = {
            let mut rng = rand::thread_rng();
            Duration::from_secs(rng.gen_range(0..20))
        };
        tokio::select! {
            _ = tokio::time::sleep(sleep_interval) => {
                let key = names.choose()?;
                let value = names.choose()?;
                tracing::debug!("Actor {:03}: adding {} to key {}", actor_id, value, key);
                let read_ctx = friend_map.len(); // we read anything from the map to get a add context
                let op = friend_map.update(
                    key.as_str(),
                    read_ctx.derive_add_ctx(actor_id),
                    |set, ctx| {
                        set.add(value, ctx)
                    }
                );
                friend_map.apply(op);
                let map_string = serde_json::to_string(&friend_map)?;
                broadcast_tx.send(map_string)?;
            }
            _ = halt_rx.changed() => {halt = *halt_rx.borrow()}
            recv_result = broadcast_rx.recv() => {
                match recv_result {
                    Ok(data) => {
                        tracing::debug!("actor: {:03}; received {} bytes", actor_id, data.len());
                        let incoming_map: FriendMap = serde_json::from_str(&data)?;
                        friend_map.validate_merge(&incoming_map)?;
                        friend_map.merge(incoming_map);
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
