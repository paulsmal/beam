# Beam

An HTTP streaming proxy server built with Rust and Axum that enables real-time file streaming between upload and download clients.

## Overview

Beam acts as a streaming bridge between clients, allowing one client to upload a file while another simultaneously downloads it. The server holds no persistent storage - it simply pipes data from uploader to downloader in real-time.

## How It Works

1. Start the server with a username and password (`beam <username> <password>`)
2. An authenticated upload client sends a `PUT` request with Basic Auth to `/{filename}`
3. The server creates an in-memory channel for streaming data and waits for a downloader
4. A download client authenticates with the same credentials and performs a `GET /{filename}`
5. The server pipes data from the uploader to the downloader in real time
6. When the upload completes or either side disconnects, the stream is torn down

### Starting the server

```bash
cargo run -- <username> <password>
# Or if built:
./target/release/beam <username> <password>
```

The server will start on `http://127.0.0.1:4000` and require the credentials you provided.

#### Endpoints
- **GET** `/` - Dashboard showing active streams
- **PUT** `/{filename}` - Upload a file using HTTP Basic Auth
- **GET** `/{filename}` - Download the active stream with the same credentials

### Example Usage

Launch the server, then upload and download in separate terminals using the same credentials:

**Terminal 1 (Uploader):**
```bash
curl -u alice:secret123 -T myfile.zip http://localhost:4000/myfile.zip
```

**Terminal 2 (Downloader):**
```bash
curl -u alice:secret123 http://localhost:4000/myfile.zip -o myfile.zip
```

The file streams directly from the uploader to the downloader without touching disk.

## Architecture

The application uses:
- **Axum**: Web framework for handling HTTP requests
- **Tokio**: Async runtime for concurrent operations
- **Tokio mpsc channels**: For streaming data between upload and download handlers
- **RwLock<HashMap>**: Shared in-memory state for active streams

## Features

- **Basic authentication**: Username/password credentials protect uploads and downloads
- **Ephemeral streams**: Data stays in memory and only exists while both clients are connected
- **Stream isolation**: Each filename can be streamed by one uploader at a time

## Limitations

- No persistent storage - files only exist during active streaming
- Credentials are stored in-memory and cleared when the server restarts
- One upload per filename at a time
- No resume capability for interrupted transfers
- Upload waits up to 5 minutes for a download client to connect

### Running tests

```bash
cargo test
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Security Considerations

This is a prototype implementation. For production use, consider adding:
- Persistent credential storage beyond the in-memory map
- Role-based authorization and audit logging
- Rate limiting and abuse prevention
- File size limits and stricter streaming guards
- HTTPS termination and secure password policies
- Input validation for filenames
