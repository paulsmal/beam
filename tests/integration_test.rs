
use anyhow::Result;
use beam_gemini::setup_server;
use reqwest;
use tokio;

#[tokio::test]
async fn test_upload_download_stream() -> Result<()> {
    let server_handle = setup_server().await;

    // Give the server a moment to start up
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let file_name = "test_file.txt";
    let upload_content = "Hello, world!";

    // 1. Start a download request
    let download_url = format!("http://localhost:3000/{}", file_name);
    let download_response_future = client.get(&download_url).send();

    // Give the server a moment to set up the stream
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 2. Start an upload request
    let upload_url = format!("http://localhost:3000/{}", file_name);
    let upload_response_future = client
        .put(&upload_url)
        .body(upload_content)
        .send();

    // 3. Await both responses
    let (download_response, upload_response) =
        tokio::join!(download_response_future, upload_response_future);

    // 4. Verify responses and content
    let download_response = download_response?;
    assert_eq!(download_response.status(), reqwest::StatusCode::OK);

    let upload_response = upload_response?;
    assert_eq!(upload_response.status(), reqwest::StatusCode::OK);

    let downloaded_content = download_response.text().await?;
    assert_eq!(downloaded_content, upload_content);

    server_handle.abort();

    Ok(())
}
