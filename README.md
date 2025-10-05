# Beam

An HTTP streaming proxy server built with Rust and Axum that enables real-time file streaming between upload and download clients.

## Overview

Beam acts as a streaming bridge between clients, allowing one client to upload a file while another simultaneously downloads it. The server holds no persistent storage - it simply pipes data from uploader to downloader in real-time.

## How It Works

1. An upload client sends a PUT request to `/{filename}`
2. The server creates a broadcast channel for streaming data
3. The upload waits for a download client to connect
4. A download client initiates a GET request to the same `/{filename}` endpoint
5. The server pipes data from the upload stream directly to the download stream
6. Once the upload completes or either client disconnects, the stream ends

### Starting the server

```bash
cargo run
# Or if built:
./target/release/beam
```

The server will start on `http://0.0.0.0:4000`

#### Endpoints
- **GET** `/` - Shows active streams and tokens
- **POST** `/token` - Generate a new authentication token
- **PUT** `/{token}/{filename}` - Upload a file (waits for download client)
- **GET** `/{token}/{filename}` - Download the streaming file

### Example Usage

First, generate a token:
```bash
TOKEN=$(curl -X POST http://localhost:4000/token)
echo "Token: $TOKEN"
```

Then use the token for upload and download in separate terminals:

**Terminal 1 (Uploader):**
```bash
curl -T myfile.zip http://localhost:4000/$TOKEN/myfile.zip
```

**Terminal 2 (Downloader):**
```bash
curl http://localhost:4000/$TOKEN/myfile.zip > myfile.zip
```

The file will stream directly from the uploader to the downloader.

## Architecture

The application uses:
- **Axum**: Web framework for handling HTTP requests
- **Tokio**: Async runtime for concurrent operations
- **Broadcast channels**: For streaming data between upload and download handlers
- **DashMap**: Thread-safe concurrent HashMap for managing active streams

## Features

- **Token-based authentication**: Secure access with short-lived tokens
- **Automatic token extension**: Tokens extend while actively streaming
- **Token cleanup**: Expired tokens are automatically removed
- **Stream isolation**: Each token's uploads/downloads are isolated

## Token Lifecycle

- Initial lifetime: 20 minutes
- Extended by 5 minutes on each use
- Tokens expire if unused
- Active streams keep tokens alive

## Limitations

- No persistent storage - files only exist during active streaming
- One upload per filename per token at a time
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
- Authentication and authorization
- Rate limiting
- File size limits
- Connection timeouts
- HTTPS support
- Input validation for filenames
