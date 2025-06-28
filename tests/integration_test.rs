use anyhow::Result;
use beam::setup_server_with_port;
use reqwest;
use tokio;

#[tokio::test]
async fn test_upload_download_stream() -> Result<()> {
    let server_handle = setup_server_with_port(3001).await;

    // Give the server a moment to start up
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let file_name = "test_file.txt";
    let upload_content = "Hello, world!";

    // 1. Start an upload request first
    let upload_url = format!("http://localhost:3001/{}", file_name);
    let upload_response_future = client.put(&upload_url).body(upload_content).send();

    // Give the server a moment to set up the stream
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 2. Start a download request
    let download_url = format!("http://localhost:3001/{}", file_name);
    let download_response_future = client.get(&download_url).send();

    // 3. Await both responses
    let (upload_response, download_response) =
        tokio::join!(upload_response_future, download_response_future);

    // 4. Verify responses and content
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
    let server_handle = setup_server_with_port(3002).await;

    // Give the server a moment to start up
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let file_name = "nonexistent_file.txt";

    // Try to download a file that hasn't been uploaded
    let download_url = format!("http://localhost:3002/{}", file_name);
    let download_response = client.get(&download_url).send().await?;

    // Should return 404
    assert_eq!(download_response.status(), reqwest::StatusCode::NOT_FOUND);

    server_handle.abort();

    Ok(())
}

#[tokio::test]
async fn test_upload_download_binary_file() -> Result<()> {
    let server_handle = setup_server_with_port(3003).await;

    // Give the server a moment to start up
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let file_name = "test_binary.bin";

    // Create some binary data (simulating a small image or binary file)
    let mut binary_content = vec![0x89, 0x50, 0x4E, 0x47]; // PNG header
    binary_content.extend_from_slice(&[0x0D, 0x0A, 0x1A, 0x0A]);
    // Add some random binary data
    for i in 0..1024 * 100 {
        binary_content.push((i % 256) as u8);
    }

    // 1. Start an upload request first
    let upload_url = format!("http://localhost:3003/{}", file_name);
    let upload_response_future = client.put(&upload_url).body(binary_content.clone()).send();

    // Give the server a moment to set up the stream
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 2. Start a download request
    let download_url = format!("http://localhost:3003/{}", file_name);
    let download_response_future = client.get(&download_url).send();

    // 3. Await both responses
    let (upload_response, download_response) =
        tokio::join!(upload_response_future, download_response_future);

    // 4. Verify responses and content
    let upload_response = upload_response?;
    assert_eq!(upload_response.status(), reqwest::StatusCode::OK);

    let download_response = download_response?;
    assert_eq!(download_response.status(), reqwest::StatusCode::OK);

    let downloaded_bytes = download_response.bytes().await?;
    assert_eq!(downloaded_bytes.to_vec(), binary_content);

    server_handle.abort();

    Ok(())
}
