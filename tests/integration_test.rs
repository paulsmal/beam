use anyhow::Result;
use beam::setup_server_with_port;
use reqwest;
use tokio;

type Port = u16;

#[tokio::test]
async fn test_upload_download_stream() -> Result<()> {
    let port: Port = 3001;
    let username = "alice";
    let password = "secret123";

    let server_handle = setup_server_with_port(port, username, password).await;
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();

    let file_name = "test_file.txt";
    let upload_content = "Hello, world!";

    let upload_url = format!("http://localhost:{port}/{}", file_name);
    let upload_response_future = client
        .put(&upload_url)
        .basic_auth(username, Some(password))
        .body(upload_content)
        .send();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let download_url = format!("http://localhost:{port}/{}", file_name);
    let download_response_future = client
        .get(&download_url)
        .basic_auth(username, Some(password))
        .send();

    let (upload_response, download_response) =
        tokio::join!(upload_response_future, download_response_future);

    let upload_response = upload_response?;
    assert_eq!(upload_response.status(), reqwest::StatusCode::OK);

    let download_response = download_response?;
    assert_eq!(download_response.status(), reqwest::StatusCode::OK);

    let downloaded_content = download_response.text().await?;
    assert_eq!(downloaded_content, upload_content);

    server_handle.abort();

    Ok(())
}

#[tokio::test]
async fn test_download_without_upload_returns_404() -> Result<()> {
    let port: Port = 3002;
    let username = "bob";
    let password = "hunter2";

    let server_handle = setup_server_with_port(port, username, password).await;
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let file_name = "nonexistent_file.txt";
    let download_url = format!("http://localhost:{port}/{}", file_name);
    let download_response = client
        .get(&download_url)
        .basic_auth(username, Some(password))
        .send()
        .await?;

    assert_eq!(download_response.status(), reqwest::StatusCode::NOT_FOUND);

    server_handle.abort();

    Ok(())
}

#[tokio::test]
async fn test_upload_download_binary_file() -> Result<()> {
    let port: Port = 3003;
    let username = "carol";
    let password = "sup3rsecret";

    let server_handle = setup_server_with_port(port, username, password).await;
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let file_name = "test_binary.bin";

    let mut binary_content = vec![0x89, 0x50, 0x4E, 0x47];
    binary_content.extend_from_slice(&[0x0D, 0x0A, 0x1A, 0x0A]);
    for i in 0..(1024 * 100) {
        binary_content.push((i % 256) as u8);
    }

    let upload_url = format!("http://localhost:{port}/{}", file_name);
    let upload_response_future = client
        .put(&upload_url)
        .basic_auth(username, Some(password))
        .body(binary_content.clone())
        .send();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let download_url = format!("http://localhost:{port}/{}", file_name);
    let download_response_future = client
        .get(&download_url)
        .basic_auth(username, Some(password))
        .send();

    let (upload_response, download_response) =
        tokio::join!(upload_response_future, download_response_future);

    let upload_response = upload_response?;
    assert_eq!(upload_response.status(), reqwest::StatusCode::OK);

    let download_response = download_response?;
    assert_eq!(download_response.status(), reqwest::StatusCode::OK);

    let downloaded_bytes = download_response.bytes().await?;
    assert_eq!(downloaded_bytes.to_vec(), binary_content);

    server_handle.abort();

    Ok(())
}
