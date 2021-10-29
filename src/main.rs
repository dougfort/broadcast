use anyhow::Error;

mod signal;

#[tokio::main]
async fn main()  -> Result<(), Error> {
    // Set the RUST_LOG, if it hasn't been explicitly defined
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "broadcast=debug")
    }
    tracing_subscriber::fmt::init();

    const ACTOR_COUNT: usize = 2;

    tracing::info!("program starts");

    let mut join_handles = vec![];
    for actor_id in 1..=ACTOR_COUNT {
        let join_handle = tokio::spawn(async move {
            actor(actor_id).await;
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

async fn actor(actor_id: usize) {
    tracing::debug!("actor {}", actor_id);
}
