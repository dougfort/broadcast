use anyhow::Error;

mod signal;

#[tokio::main]
async fn main()  -> Result<(), Error> {
    // Set the RUST_LOG, if it hasn't been explicitly defined
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "broadcast=debug")
    }
    tracing_subscriber::fmt::init();

    tracing::info!("program starts");

    tracing::info!("waiting for shutdown signal");
    signal::shutdown_signal().await;

    tracing::info!("progam terminates normally");
    Ok(())
}
