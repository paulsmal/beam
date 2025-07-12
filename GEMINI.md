## Project: beam

**Description:** This project is a file streaming server written in Rust. It allows a user to upload a file and download it simultaneously, without storing the file on the server. It uses the Axum web framework and Tokio for asynchronous operations.

**How to run:**
1. Build the project: `cargo build`
2. Run the server: `cargo run`
3. In a separate terminal, upload a file: `curl -T path/to/your/file http://localhost:4000/your-file-name`
4. In another terminal, download the file: `wget http://localhost:4000/your-file-name`

**Key files:**
* `src/main.rs`: The entry point of the application. It sets up the server and logging.
* `src/lib.rs`: Contains the core logic of the server, including the Axum routes and handlers for uploading and downloading files.
* `Cargo.toml`: The package manifest for the Rust project. It contains the dependencies and metadata for the project.

**Architecture:**
The server uses an in-memory `HashMap` to store active streams. When a file is uploaded, a new entry is created in the `HashMap` with a unique file ID. The server then waits for a download request with the same file ID. Once the download request is received, the server starts streaming the file from the upload stream to the download stream. The file is never stored on the server's disk.
