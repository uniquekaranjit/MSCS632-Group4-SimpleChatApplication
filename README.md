# MSCS632-Group4-SimpleChatApplication
# Simple Chat App implemented in Go and Rust. 
# Simple TCP Chat Application

A multi-client TCP chat application implemented in both Rust and Go. The application supports real-time messaging, user search, and keyword search functionality.

## Features

- Real-time multi-client chat
- User registration with unique IDs
- Message history storage
- Search messages by keyword
- Search messages by username
- Join/Leave notifications
- Simple command interface

## Prerequisites

### Rust Version
- Rust 1.70 or later
- Cargo (Rust's package manager)

### Go Version
- Go 1.16 or later

## Dependencies

### Rust
Add the following to your `Cargo.toml`:

```toml
[dependencies]
tokio = { version = "1.36", features = ["full"] }
uuid = { version = "1" }
chrono = "0.4"
```

## Running the Application

### Rust Version
1. Navigate to the Rust project directory:

```bash
cd SimpleRustChatApp
```

2. Run the server:
```bash
cargo run
```

3. Connect clients using telnet:
```bash
telnet 127.0.0.1 8080
```

### Go Version
1. Navigate to the Go project directory:
```bash
cd SimpleGoChatApp
```

2. Run the server:
```bash
go run main.go
```

3. Connect clients using telnet:
```bash
telnet 127.0.0.1 8080
```

## Available Commands

- `exit` - Leave the chat
- `/search <query>` - Search messages by keyword
- `/user <username>` - Search messages by username
- Any other message - Send a chat message

## Implementation Details

### Rust Version
- Uses Tokio for async I/O
- Implements thread-safe message storage with Arc<Mutex>
- Uses broadcast channels for message distribution
- Handles concurrent connections efficiently

### Go Version
- Uses goroutines for concurrent connections
- Implements thread-safe message storage with sync.Mutex
- Uses channels for message distribution
- Handles concurrent connections with goroutines

