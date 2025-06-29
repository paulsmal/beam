use axum::{
    body::{Body, Bytes},
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use futures_util::stream::StreamExt;
use http_body::Frame;
use http_body_util::{BodyStream, StreamBody};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::ReceiverStream;

type FileId = String;

struct StreamData {
    receiver: mpsc::Receiver<Result<Bytes, axum::Error>>,
    ready_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

#[derive(Clone, Default)]
struct AppState {
    streams: Arc<RwLock<HashMap<FileId, StreamData>>>,
}


pub async fn setup_server() -> tokio::task::JoinHandle<()> {
    setup_server_with_port(3000).await
}

pub async fn setup_server_with_port(port: u16) -> tokio::task::JoinHandle<()> {
    let state = AppState::default();

    let app = Router::new()
        .route("/", get(dashboard))
        .route(
            "/{filename}",
            get(download_handler).put(upload_handler),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();
    tracing::info!("Listening on {}", listener.local_addr().unwrap());
    
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    })
}

async fn dashboard(State(state): State<AppState>) -> impl IntoResponse {
    let keys = state.streams.read().await.keys().cloned().collect::<Vec<_>>();
    format!("Streams waiting for download: {keys:?}")
}

async fn download_handler(
    State(state): State<AppState>,
    Path(filename): Path<FileId>,
) -> Response<Body> {
    let stream_data = match state.streams.write().await.remove(&filename) {
        Some(mut data) => {
            // Signal the upload handler that download client is ready
            if let Some(ready_tx) = data.ready_tx.take() {
                let _ = ready_tx.send(());
            }
            data
        },
        None => {
            tracing::warn!(%filename, "Download rejected: no active upload stream.");
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("No active upload stream for this file"))
                .unwrap();
        }
    };

    tracing::info!(%filename, "Download started, streaming from upload.");

    let receiver_stream = ReceiverStream::new(stream_data.receiver);
    let stream_body = StreamBody::new(receiver_stream.map(|res| res.map(Frame::data)));

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_DISPOSITION, format!("attachment; filename=\"{filename}\""))
        .body(Body::new(stream_body))
        .unwrap()
}

async fn upload_handler(
    State(state): State<AppState>,
    Path(filename): Path<FileId>,
    body: Body,
) -> impl IntoResponse {
    let (tx, rx) = mpsc::channel(16);
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let (complete_tx, complete_rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

    // Store the receiver and ready signal
    state.streams.write().await.insert(filename.clone(), StreamData {
        receiver: rx,
        ready_tx: Some(ready_tx),
    });
    tracing::info!(%filename, "Upload connection accepted. Waiting for download client.");

    tokio::spawn(async move {
        // Wait for download client to connect
        match tokio::time::timeout(Duration::from_secs(300), ready_rx).await {
            Ok(Ok(())) => {
                tracing::info!(%filename, "Download client connected, starting to read upload stream");
            }
            Ok(Err(_)) => {
                tracing::warn!(%filename, "Ready channel dropped without signal");
                let _ = complete_tx.send(Err("Ready channel dropped".to_string()));
                return;
            }
            Err(_) => {
                tracing::warn!(%filename, "Upload timed out waiting for download client (300s)");
                let _ = complete_tx.send(Err("Timeout waiting for download client".to_string()));
                return;
            }
        }

        // Now start reading the body
        let mut body_stream = BodyStream::new(body);

        while let Some(chunk_result) = body_stream.next().await {
            match chunk_result {
                Ok(frame) => {
                    if let Ok(bytes) = frame.into_data() {
                        if tx.send(Ok(bytes)).await.is_err() {
                            tracing::info!(%filename, "Download client disconnected. Stopping upload.");
                            break;
                        }
                    }
                }
                Err(e) => {
                    let error_msg = format!("Stream error: {e}");
                    tracing::error!(%filename, "Error reading upload stream: {}", e);
                    let _ = tx.send(Err(e)).await;
                    let _ = complete_tx.send(Err(error_msg));
                    return;
                }
            }
        }
        tracing::info!(%filename, "Upload stream finished.");
        let _ = complete_tx.send(Ok(()));
    });

    // Wait for the upload to complete before returning response
    match complete_rx.await {
        Ok(Ok(())) => (StatusCode::OK, "Upload completed successfully").into_response(),
        Ok(Err(e)) => (StatusCode::BAD_REQUEST, format!("Upload failed: {e}")).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Upload task failed").into_response(),
    }
}
