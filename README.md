# Beam Gemini

A lightweight HTTP streaming proxy server built with Rust and Axum that enables real-time file streaming between upload and download clients.

## Overview

Beam Gemini acts as a streaming bridge between clients, allowing one client to upload a file while another simultaneously downloads it. The server holds no persistent storage - it simply pipes data from uploader to downloader in real-time.

## Features

- **Real-time streaming**: Data flows directly from upload to download without intermediate storage
- **Memory efficient**: Uses async streams and channels to handle large files without loading them into memory
- **Simple REST API**: Easy-to-use HTTP endpoints for uploading and downloading
- **Dashboard**: Basic web interface to view active streams
- **Concurrent handling**: Built on Tokio for handling multiple simultaneous streams

## How It Works

1. A download client initiates a GET request to `/{filename}`
2. The server creates a channel and waits for an upload
3. An upload client sends a PUT request to the same `/{filename}` endpoint
4. The server pipes data from the upload stream directly to the download stream
5. Once the upload completes or either client disconnects, the stream ends

## Installation

### Prerequisites

- Rust 1.70 or higher
- Cargo

### Building from source

```bash
git clone https://github.com/yourusername/beam-gemini.git
cd beam-gemini
cargo build --release
```

## Usage

### Starting the server

```bash
cargo run
# Or if built:
./target/release/beam-gemini
```

The server will start on `http://0.0.0.0:3000`

### API Endpoints

#### Dashboard
- **GET** `/` - Shows list of active streams waiting for upload

#### File Streaming
- **GET** `/{filename}` - Initiate a download stream (waits for corresponding upload)
- **PUT** `/{filename}` - Upload a file to stream to waiting download client

### Example Usage

Using curl in two separate terminals:

**Terminal 1 (Downloader):**
```bash
curl -O http://localhost:3000/myfile.zip
```

**Terminal 2 (Uploader):**
```bash
curl -X PUT --data-binary @myfile.zip http://localhost:3000/myfile.zip
```

The file will stream directly from the uploader to the downloader.

## Architecture

The application uses:
- **Axum**: Web framework for handling HTTP requests
- **Tokio**: Async runtime for concurrent operations
- **Channels**: MPSC channels for streaming data between upload and download handlers
- **RwLock**: Thread-safe storage for managing active streams

## Configuration

Currently, the server uses hardcoded configuration:
- **Port**: 3000
- **Host**: 0.0.0.0 (all interfaces)
- **Channel buffer size**: 16 chunks

## Limitations

- No authentication or authorization
- No persistent storage - if download client disconnects, upload must restart
- One upload per filename at a time
- No resume capability for interrupted transfers

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