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

The server will start on `http://0.0.0.0:3000`

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

## Limitations

- No authentication or authorization
- No persistent storage - files only exist during active streaming
- One upload per filename at a time (returns 409 Conflict for concurrent uploads)
- No resume capability for interrupted transfers
- Upload waits indefinitely for a download client to connect

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
