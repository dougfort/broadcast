use anyhow::Error;
use rand::Rng;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, watch};

mod signal;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Set the RUST_LOG, if it hasn't been explicitly defined
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "broadcast=debug")
    }
    tracing_subscriber::fmt::init();

    const ACTOR_COUNT: usize = 10;
    const BROADCAST_CHANNEL_SIZE: usize = ACTOR_COUNT;

    tracing::info!("program starts");

    let (broadcast_tx, _) = broadcast::channel::<String>(BROADCAST_CHANNEL_SIZE);
    let broadcast_tx = Arc::new(broadcast_tx);

    let mut halt = false;
    let (halt_tx, halt_rx) = watch::channel(halt);

    let mut join_handles = vec![];
    for actor_id in 1..=ACTOR_COUNT {
        let broadcast_tx = broadcast_tx.clone();
        let broadcast_rx = broadcast_tx.subscribe();
        let halt_rx = halt_rx.clone();
        let join_handle = tokio::spawn(async move {
            actor(actor_id, broadcast_tx, broadcast_rx, halt_rx).await?;

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
    broadcast_tx: Arc<broadcast::Sender<String>>,
    mut broadcast_rx: broadcast::Receiver<String>,
    mut halt_rx: watch::Receiver<bool>,
) -> Result<(), Error> {
    let mut halt = *halt_rx.borrow();
    let mut count = 0;

    while !halt {
        let sleep_interval = {
            let mut rng = rand::thread_rng();
            Duration::from_secs(rng.gen_range(0..20))
        };
        tokio::select! {
            _ = tokio::time::sleep(sleep_interval) => {
                count += 1;
                broadcast_tx.send(format!("{:>5} from actor {:03}", count, actor_id))?;
            }
            _ = halt_rx.changed() => {halt = *halt_rx.borrow()}
            recv_result = broadcast_rx.recv() => {
                match recv_result {
                    Ok(data) => {
                        tracing::debug!("actor: {:03}; data = {}", actor_id, data)
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
