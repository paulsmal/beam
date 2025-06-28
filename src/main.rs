use beam_gemini::setup_server;
use tracing;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let server_handle = setup_server().await;
    server_handle.await.unwrap();
}