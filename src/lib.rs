use axum::{
    Router,
    body::{Body, Bytes},
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use futures_util::stream::StreamExt;
use http_body::Frame;
use http_body_util::{BodyStream, StreamBody};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::{RwLock, mpsc};
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

type FileId = String;
type Token = String;

struct StreamData {
    receiver: mpsc::Receiver<Result<Bytes, axum::Error>>,
    ready_tx: Option<tokio::sync::oneshot::Sender<()>>,
    token: Token,
}

#[derive(Clone)]
struct TokenData {
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    active_streams: usize,
}

#[derive(Clone, Default)]
struct AppState {
    streams: Arc<RwLock<HashMap<FileId, StreamData>>>,
    tokens: Arc<RwLock<HashMap<Token, TokenData>>>,
}

const TOKEN_LIFETIME_MINUTES: i64 = 20;
const TOKEN_EXTENSION_MINUTES: i64 = 5;

pub async fn setup_server() -> tokio::task::JoinHandle<()> {
    setup_server_with_port(4000).await
}

pub async fn setup_server_with_port(port: u16) -> tokio::task::JoinHandle<()> {
    let state = AppState::default();

    let app = Router::new()
        .route("/", get(dashboard))
        .route("/token", post(create_token))
        .route(
            "/{token}/{filename}",
            get(download_handler).put(upload_handler),
        )
        .with_state(state.clone());

    // Start token cleanup task
    tokio::spawn(cleanup_expired_tokens(state));

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();
    tracing::info!("Listening on {}", listener.local_addr().unwrap());

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    })
}

async fn cleanup_expired_tokens(state: AppState) {
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;

        let now = Utc::now();
        let mut tokens = state.tokens.write().await;
        tokens.retain(|_, token_data| token_data.expires_at > now);
    }
}

async fn dashboard(State(state): State<AppState>) -> impl IntoResponse {
    let streams = state.streams.read().await;
    let tokens = state.tokens.read().await;

    let active_streams = streams.keys().cloned().collect::<Vec<_>>();

    let token_count = tokens.len();

    format!("Active streams: {active_streams:?}\nActive tokens: {token_count}")
}

async fn create_token(State(state): State<AppState>) -> impl IntoResponse {
    let token = Uuid::new_v4().to_string();
    let now = Utc::now();

    let token_data = TokenData {
        created_at: now,
        expires_at: now + chrono::Duration::minutes(TOKEN_LIFETIME_MINUTES),
        active_streams: 0,
    };

    state.tokens.write().await.insert(token.clone(), token_data);

    tracing::info!("Created new token: {}", &token);
    (StatusCode::OK, token)
}

async fn validate_and_extend_token(state: &AppState, token: &str) -> Result<(), StatusCode> {
    let mut tokens = state.tokens.write().await;

    match tokens.get_mut(token) {
        Some(token_data) => {
            let now = Utc::now();
            if token_data.expires_at < now {
                tracing::warn!("Token {} has expired", token);
                return Err(StatusCode::UNAUTHORIZED);
            }

            // Extend token lifetime when actively used
            token_data.expires_at = now + chrono::Duration::minutes(TOKEN_EXTENSION_MINUTES);
            Ok(())
        }
        None => {
            tracing::warn!("Invalid token: {}", token);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

async fn download_handler(
    State(state): State<AppState>,
    Path((token, filename)): Path<(String, String)>,
) -> Response<Body> {
    // Validate token
    if let Err(status) = validate_and_extend_token(&state, &token).await {
        return Response::builder()
            .status(status)
            .body(Body::from("Invalid or expired token"))
            .unwrap();
    }

    let stream_data = match state.streams.write().await.remove(&filename) {
        Some(mut data) => {
            // Verify this stream belongs to the token
            if data.token != token {
                tracing::warn!("Token mismatch for file {}", filename);
                return Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .body(Body::from("Token does not match upload"))
                    .unwrap();
            }

            // Signal the upload handler that download client is ready
            if let Some(ready_tx) = data.ready_tx.take() {
                let _ = ready_tx.send(());
            }

            // Decrement active streams for this token
            if let Some(token_data) = state.tokens.write().await.get_mut(&token) {
                token_data.active_streams = token_data.active_streams.saturating_sub(1);
            }

            data
        }
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
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        )
        .body(Body::new(stream_body))
        .unwrap()
}

async fn upload_handler(
    State(state): State<AppState>,
    Path((token, filename)): Path<(String, String)>,
    body: Body,
) -> impl IntoResponse {
    // Validate token
    if let Err(status) = validate_and_extend_token(&state, &token).await {
        return (status, "Invalid or expired token").into_response();
    }

    // Increment active streams for this token
    {
        let mut tokens = state.tokens.write().await;
        if let Some(token_data) = tokens.get_mut(&token) {
            token_data.active_streams += 1;
        }
    }

    let (tx, rx) = mpsc::channel(16);
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let (complete_tx, complete_rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

    // Store the receiver and ready signal
    state.streams.write().await.insert(
        filename.clone(),
        StreamData {
            receiver: rx,
            ready_tx: Some(ready_tx),
            token: token.clone(),
        },
    );

    tracing::info!(%filename, %token, "Upload connection accepted. Waiting for download client.");

    let state_clone = state.clone();
    let token_clone = token.clone();

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

                        // Extend token lifetime while actively streaming
                        let _ = validate_and_extend_token(&state_clone, &token_clone).await;
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

