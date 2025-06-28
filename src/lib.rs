use axum::{
    body::{Body, Bytes},
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use futures_util::stream::StreamExt;
use http_body::Frame;
use http_body_util::{BodyStream, StreamBody};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::ReceiverStream;
use tracing;

type FileId = String;
type StreamChannel = mpsc::Sender<Result<Bytes, axum::Error>>;

#[derive(Clone, Default)]
struct AppState {
    streams: Arc<RwLock<HashMap<FileId, StreamChannel>>>,
}


pub async fn setup_server() -> tokio::task::JoinHandle<()> {
    let state = AppState::default();

    let app = Router::new()
        .route("/", get(dashboard))
        .route(
            "/{filename}",
            get(download_handler).put(upload_handler),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("Listening on {}", listener.local_addr().unwrap());
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    handle
}

async fn dashboard(State(state): State<AppState>) -> impl IntoResponse {
    let keys = state.streams.read().await.keys().cloned().collect::<Vec<_>>();
    format!("Streams waiting for upload: {:?}", keys)
}

async fn download_handler(
    State(state): State<AppState>,
    Path(filename): Path<FileId>,
) -> impl IntoResponse {
    let (tx, rx) = mpsc::channel(16);
    state.streams.write().await.insert(filename.clone(), tx);
    tracing::info!(%filename, "Download stream initiated. Waiting for upload.");

    let receiver_stream = ReceiverStream::new(rx);
    let stream_body = StreamBody::new(receiver_stream.map(|res| res.map(Frame::data)));

    Response::builder()
        .status(StatusCode::OK)
        .body(stream_body)
        .unwrap()
}

async fn upload_handler(
    State(state): State<AppState>,
    Path(filename): Path<FileId>,
    body: Body,
) -> impl IntoResponse {
    let tx = match state.streams.write().await.remove(&filename) {
        Some(tx) => tx,
        None => {
            tracing::warn!(%filename, "Upload rejected: no active download stream.");
            return (
                StatusCode::NOT_FOUND,
                "No active download stream for this file",
            )
                .into_response();
        }
    };

    tracing::info!(%filename, "Upload started, piping to download stream.");

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
                tracing::error!(%filename, "Error reading upload stream: {}", e);
                let _ = tx.send(Err(e)).await;
                break;
            }
        }
    }

    tracing::info!(%filename, "Upload finished.");
    (StatusCode::OK, "Upload complete").into_response()
}
