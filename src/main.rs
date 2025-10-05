use beam::setup_server;
use std::env;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let mut args = env::args().skip(1);
    let username = args
        .next()
        .unwrap_or_else(|| usage_and_exit("missing <username> argument"));
    let password = args
        .next()
        .unwrap_or_else(|| usage_and_exit("missing <password> argument"));

    if args.next().is_some() {
        usage_and_exit("too many arguments");
    }

    let server_handle = setup_server(&username, &password).await;
    server_handle.await.unwrap();
}

fn usage_and_exit(msg: &str) -> ! {
    eprintln!("Error: {msg}");
    eprintln!("Usage: beam <username> <password>");
    std::process::exit(1);
}
