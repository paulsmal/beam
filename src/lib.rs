use axum::{
    Router,
    body::{Body, Bytes},
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use futures_util::stream::StreamExt;
use http_body::Frame;
use http_body_util::{BodyStream, StreamBody};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::{RwLock, mpsc};
use tokio_stream::wrappers::ReceiverStream;

use argon2::password_hash::{SaltString, rand_core::OsRng};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use headers::{Authorization, Header, authorization::Basic};
use tracing::{error, info, warn};

pub async fn setup_server(username: &str, password: &str) -> tokio::task::JoinHandle<()> {
    setup_server_with_port(4000, username, password).await
}

pub async fn setup_server_with_port(
    port: u16,
    username: &str,
    password: &str,
) -> tokio::task::JoinHandle<()> {
    let auth = AuthConfig::new(username, password).expect("failed to hash startup password");
    let state = AppState::new(auth);

    let app = Router::new()
        .route("/", get(dashboard))
        .route("/{filename}", get(download_handler).put(upload_handler))
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("failed to bind TCP listener");
    info!("Listening on {}", listener.local_addr().unwrap());

    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("server task failed");
    })
}

#[derive(Clone)]
struct AppState {
    streams: Arc<RwLock<HashMap<String, StreamData>>>,
    auth: Arc<AuthConfig>,
}

impl AppState {
    fn new(auth: AuthConfig) -> Self {
        Self {
            streams: Arc::new(RwLock::new(HashMap::new())),
            auth: Arc::new(auth),
        }
    }
}

struct AuthConfig {
    username: String,
    password_hash: String,
}

impl AuthConfig {
    fn new(username: &str, password: &str) -> Result<Self, argon2::password_hash::Error> {
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)?
            .to_string();

        Ok(Self {
            username: username.to_owned(),
            password_hash,
        })
    }
}

struct StreamData {
    receiver: mpsc::Receiver<Result<Bytes, axum::Error>>,
    ready_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

#[derive(Debug)]
enum AuthError {
    Unauthorized,
    Internal,
}

fn auth_error_response(error: AuthError) -> Response<Body> {
    match error {
        AuthError::Unauthorized => unauthorized_response("Invalid username or password"),
        AuthError::Internal => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("Authentication failed"))
            .expect("failed to build auth error response"),
    }
}

fn unauthorized_response(message: &str) -> Response<Body> {
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(header::WWW_AUTHENTICATE, "Basic realm=\"beam\"")
        .body(Body::from(message.to_owned()))
        .expect("failed to build unauthorized response")
}

fn extract_basic_auth(headers: &HeaderMap) -> Result<Authorization<Basic>, AuthError> {
    let Some(header_value) = headers.get(header::AUTHORIZATION) else {
        warn!("Missing Authorization header");
        return Err(AuthError::Unauthorized);
    };

    let mut values = std::iter::once(header_value);
    Authorization::<Basic>::decode(&mut values).map_err(|error| {
        warn!(%error, "Failed to parse Authorization header");
        AuthError::Unauthorized
    })
}

async fn authenticate_user(state: &AppState, auth: &Authorization<Basic>) -> Result<(), AuthError> {
    let expected_username = &state.auth.username;
    let provided_username = auth.username();

    if provided_username != expected_username {
        warn!(attempted = %provided_username, "Unknown username supplied");
        return Err(AuthError::Unauthorized);
    }

    let password = auth.password();
    if password.is_empty() {
        warn!(%provided_username, "Basic auth password is empty");
        return Err(AuthError::Unauthorized);
    }

    let parsed_hash = PasswordHash::new(&state.auth.password_hash).map_err(|err| {
        error!(%provided_username, %err, "Stored password hash is invalid");
        AuthError::Internal
    })?;

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| AuthError::Unauthorized)?;

    Ok(())
}

async fn dashboard(State(state): State<AppState>) -> Html<String> {
    let streams = state.streams.read().await;
    let active_streams = streams.keys().cloned().collect::<Vec<_>>();

    let body = format!(
        r#"<!DOCTYPE html>
<html lang=\"en\">
<head>
  <meta charset=\"utf-8\" />
  <title>Beam Dashboard</title>
  <style>
    body {{ font-family: sans-serif; margin: 2rem; max-width: 40rem; }}
    h1 {{ margin-bottom: 0.5rem; }}
    section {{ margin-top: 1.5rem; }}
    code {{ background: #f4f4f4; padding: 0.2rem 0.4rem; border-radius: 3px; }}
  </style>
</head>
<body>
  <h1>Beam Dashboard</h1>
  <p>Start Beam with <code>beam &lt;username&gt; &lt;password&gt;</code> then authenticate uploads and downloads using HTTP Basic auth.</p>
  <section>
    <h2>Active Streams</h2>
    <pre>{active_streams:#?}</pre>
  </section>
  <section>
    <h2>Usage</h2>
    <ol>
      <li>Upload: <code>curl -u USER:PASS -T file.zip http://localhost:4000/file.zip</code></li>
      <li>Download: <code>curl -u USER:PASS http://localhost:4000/file.zip -o file.zip</code></li>
    </ol>
  </section>
</body>
</html>"#
    );

    Html(body)
}

async fn download_handler(
    State(state): State<AppState>,
    Path(filename): Path<String>,
    headers: HeaderMap,
) -> Response<Body> {
    let auth = match extract_basic_auth(&headers) {
        Ok(auth) => auth,
        Err(err) => return auth_error_response(err),
    };

    if let Err(err) = authenticate_user(&state, &auth).await {
        return auth_error_response(err);
    }

    let stream_data = match state.streams.write().await.remove(&filename) {
        Some(data) => data,
        None => {
            warn!(%filename, "Download rejected: no active upload");
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("No active upload stream for this file"))
                .expect("failed to build 404 response");
        }
    };

    if let Some(ready_tx) = stream_data.ready_tx {
        let _ = ready_tx.send(());
    }

    info!(%filename, "Download started");

    let receiver_stream = ReceiverStream::new(stream_data.receiver);
    let stream_body = StreamBody::new(receiver_stream.map(|res| res.map(Frame::data)));

    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        )
        .body(Body::new(stream_body))
        .expect("failed to build download response")
}

async fn upload_handler(
    State(state): State<AppState>,
    Path(filename): Path<String>,
    headers: HeaderMap,
    body: Body,
) -> impl IntoResponse {
    let auth = match extract_basic_auth(&headers) {
        Ok(auth) => auth,
        Err(err) => return auth_error_response(err),
    };

    if let Err(err) = authenticate_user(&state, &auth).await {
        return auth_error_response(err);
    }

    let (tx, rx) = mpsc::channel(16);
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let (complete_tx, complete_rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

    {
        let mut streams = state.streams.write().await;
        if streams.contains_key(&filename) {
            return (
                StatusCode::CONFLICT,
                "An upload is already in progress for this filename",
            )
                .into_response();
        }

        streams.insert(
            filename.clone(),
            StreamData {
                receiver: rx,
                ready_tx: Some(ready_tx),
            },
        );
    }

    info!(%filename, "Upload connection accepted. Waiting for download client.");

    let filename_task = filename.clone();

    tokio::spawn(async move {
        match tokio::time::timeout(Duration::from_secs(300), ready_rx).await {
            Ok(Ok(())) => {
                info!(%filename_task, "Download client connected");
            }
            Ok(Err(_)) => {
                warn!(%filename_task, "Ready channel dropped without signal");
                let _ = complete_tx.send(Err("Ready channel dropped".to_string()));
                return;
            }
            Err(_) => {
                warn!(%filename_task, "Upload timed out waiting for download client (300s)");
                let _ = complete_tx.send(Err("Timeout waiting for download client".to_string()));
                return;
            }
        }

        let mut body_stream = BodyStream::new(body);

        while let Some(chunk_result) = body_stream.next().await {
            match chunk_result {
                Ok(frame) => {
                    if let Ok(bytes) = frame.into_data() {
                        if tx.send(Ok(bytes)).await.is_err() {
                            info!(%filename_task, "Download client disconnected. Stopping upload.");
                            break;
                        }
                    }
                }
                Err(error) => {
                    let error_msg = format!("Stream error: {error}");
                    error!(%filename_task, %error, "Error reading upload stream");
                    let _ = tx.send(Err(error)).await;
                    let _ = complete_tx.send(Err(error_msg));
                    return;
                }
            }
        }

        info!(%filename_task, "Upload stream finished.");
        let _ = complete_tx.send(Ok(()));
    });

    match complete_rx.await {
        Ok(Ok(())) => {
            state.streams.write().await.remove(&filename);
            (StatusCode::OK, "Upload completed successfully").into_response()
        }
        Ok(Err(error)) => {
            state.streams.write().await.remove(&filename);
            (StatusCode::BAD_REQUEST, format!("Upload failed: {error}")).into_response()
        }
        Err(_) => {
            state.streams.write().await.remove(&filename);
            (StatusCode::INTERNAL_SERVER_ERROR, "Upload task failed").into_response()
        }
    }
}
