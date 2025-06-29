use beam::setup_server;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let server_handle = setup_server().await;
    server_handle.await.unwrap();
}
