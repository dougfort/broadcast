use anyhow::Error;
use tokio::sync::broadcast;

mod signal;

#[tokio::main]
async fn main()  -> Result<(), Error> {
    // Set the RUST_LOG, if it hasn't been explicitly defined
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "broadcast=debug")
    }
    tracing_subscriber::fmt::init();

    const ACTOR_COUNT: usize = 2;
    const BROADCAST_CHANNEL_SIZE: usize = ACTOR_COUNT;

    tracing::info!("program starts");

    let (broadcast_tx, _) = broadcast::channel::<String>(BROADCAST_CHANNEL_SIZE);

    let mut join_handles = vec![];
    for actor_id in 1..=ACTOR_COUNT {
        let broadcast_tx = broadcast_tx.clone();
        let broadcast_rx = broadcast_tx.subscribe();
        let join_handle = tokio::spawn(async move {
            actor(actor_id, broadcast_tx, broadcast_rx).await?;

            Ok::<(), Error>(())
        });
        join_handles.push(join_handle);
    }

    tracing::info!("waiting for shutdown signal");
    signal::shutdown_signal().await;

    for join_handle in join_handles {
        let result = join_handle.await?;
        tracing::debug!("join result = {:?}", result);
    }

    tracing::info!("progam terminates normally");
    Ok(())
}

async fn actor(
    actor_id: usize, 
    broadcast_tx: broadcast::Sender::<String>,
    broadcast_rx: broadcast::Receiver::<String>,
) -> Result<(), Error> {
    let mut broadcast_rx = broadcast_rx;
    broadcast_tx.send(format!("1 from actor {}", actor_id))?;
    for _ in 0..2 {
        let data = broadcast_rx.recv().await?;
        tracing::debug!("actor: {}; data = {}", actor_id, data);
    }

    tracing::debug!("actor: {} terminates", actor_id);
    Ok(())
}
