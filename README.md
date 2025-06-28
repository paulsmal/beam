# Beam

A lightweight HTTP streaming proxy server built with Rust and Axum that enables real-time file streaming between upload and download clients.

## Overview

Beam acts as a streaming bridge between clients, allowing one client to upload a file while another simultaneously downloads it. The server holds no persistent storage - it simply pipes data from uploader to downloader in real-time.

## Features

- **Real-time streaming**: Data flows directly from upload to download without intermediate storage
- **Memory efficient**: Uses async streams and channels to handle large files without loading them into memory
- **Simple REST API**: Easy-to-use HTTP endpoints for uploading and downloading
- **Dashboard**: Basic web interface to view active streams
- **Concurrent handling**: Built on Tokio for handling multiple simultaneous streams

## How It Works

1. An upload client sends a PUT request to `/{filename}`
2. The server creates a broadcast channel for streaming data
3. The upload waits for a download client to connect
4. A download client initiates a GET request to the same `/{filename}` endpoint
5. The server pipes data from the upload stream directly to the download stream
6. Once the upload completes or either client disconnects, the stream ends

## Installation

### Prerequisites

- Rust 1.70 or higher
- Cargo

### Building from source

```bash
git clone https://github.com/yourusername/beam.git
cd beam
cargo build --release
```

## Usage

### Starting the server

```bash
cargo run
# Or if built:
./target/release/beam
```

The server will start on `http://0.0.0.0:3000`

### API Endpoints

#### Dashboard
- **GET** `/` - Shows list of active streams

#### File Streaming
- **PUT** `/{filename}` - Upload a file (waits for download client before streaming)
- **GET** `/{filename}` - Download the streaming file

### Example Usage

Using curl in two separate terminals:

**Terminal 1 (Uploader):**
```bash
curl -T myfile.zip http://localhost:3000/myfile.zip
```

**Terminal 2 (Downloader):**
```bash
curl http://localhost:3000/myfile.zip > myfile.zip
```

The file will stream directly from the uploader to the downloader.

## Architecture

The application uses:
- **Axum**: Web framework for handling HTTP requests
- **Tokio**: Async runtime for concurrent operations
- **Broadcast channels**: For streaming data between upload and download handlers
- **DashMap**: Thread-safe concurrent HashMap for managing active streams

## Configuration

Currently, the server uses hardcoded configuration:
- **Port**: 3000
- **Host**: 0.0.0.0 (all interfaces)
- **Channel buffer size**: 1024 messages

## Limitations

- No authentication or authorization
- No persistent storage - files only exist during active streaming
- One upload per filename at a time (returns 409 Conflict for concurrent uploads)
- No resume capability for interrupted transfers
- Upload waits indefinitely for a download client to connect

## Development

### Running in development mode

```bash
cargo run
```

### Running tests

```bash
cargo test
```

### Logging

The application uses `tracing` for structured logging. Log level is set to INFO by default.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Security Considerations

This is a prototype implementation. For production use, consider adding:
- Authentication and authorization
- Rate limiting
- File size limits
- Connection timeouts
- HTTPS support
- Input validation for filenames
