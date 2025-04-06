// This is a Rust implementation of the described TCP chat app.
// Note: You'd run the server and clients as separate processes.

// --- Dependencies (add to Cargo.toml) ---
// tokio = { version = "1", features = ["full"] }
// uuid = { version = "1" }
// chrono = "0.4"

use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::sync::{Arc, Mutex};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use uuid::Uuid;
use chrono::Local;

#[derive(Clone)]
struct User {
    id: String,
    name: String,
}

#[derive(Clone)]
struct Message {
    sender_id: String,
    content: String,
    timestamp: String,
}

impl Message {
    fn format(&self) -> String {
        format!("[{}] {}: {}", self.timestamp, self.sender_id, self.content)
    }

    fn matches(&self, keyword: &str) -> bool {
        self.content.contains(keyword) || self.sender_id.contains(keyword)
    }
}

struct ChatManager {
    messages: Arc<Mutex<Vec<Message>>>,
    users: Arc<Mutex<HashMap<SocketAddr, User>>>,
}

impl ChatManager {
    fn new() -> Self {
        ChatManager {
            messages: Arc::new(Mutex::new(Vec::new())),
            users: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn store_message(&self, msg: Message) {
        self.messages.lock().unwrap().push(msg);
    }

    fn search_messages(&self, query: &str) -> Vec<String> {
        self.messages.lock().unwrap().iter()
            .filter(|msg| msg.matches(query))
            .map(|m| m.format())
            .collect()
    }

    fn register_user(&self, addr: SocketAddr, name: String) -> User {
        let user = User {
            id: Uuid::new_v4().to_string(),
            name: name.clone(),
        };
        self.users.lock().unwrap().insert(addr, user.clone());
        user
    }

    fn remove_user(&self, addr: &SocketAddr) {
        self.users.lock().unwrap().remove(addr);
    }
    
    // Search messages by user name
    fn search_messages_by_user(&self, user_name: &str) -> Vec<String> {
        self.messages.lock().unwrap().iter()
            .filter(|msg| msg.sender_id.contains(user_name))
            .map(|m| m.format())
            .collect()
    }

    // Search messages by keyword (content or sender name)
    fn search_messages_by_keyword(&self, keyword: &str) -> Vec<String> {
        self.messages.lock().unwrap().iter()
            .filter(|msg| msg.content.contains(keyword))
            .map(|m| m.format())
            .collect()
    }    
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    let (tx, _rx) = broadcast::channel(100);
    let chat_manager = Arc::new(ChatManager::new());

    loop {
        let (socket, addr) = listener.accept().await?;
        let tx = tx.clone();
        let mut rx = tx.subscribe();
        let chat_manager = chat_manager.clone();

        tokio::spawn(async move {
            let (reader, mut writer) = socket.into_split();
            let mut reader = BufReader::new(reader);
            let mut line = String::new();

            // Get user's name
            writer.write_all(b"Enter your name: ").await.unwrap();
            writer.flush().await.unwrap();
            reader.read_line(&mut line).await.unwrap();
            let name = line.trim().to_string();
            let user = chat_manager.register_user(addr, name);
            line.clear();

            // Show command instructions in a box
            writer.write_all(b"\n\n***************************************************\n").await.unwrap();
            writer.write_all(b"- Type 'exit' to leave\n").await.unwrap();
            writer.write_all(b"- Type '/search <query>' to search by keyword\n").await.unwrap();
            writer.write_all(b"- Type '/user <username>' to search by user\n").await.unwrap();
            writer.write_all(b"- Type any other message to chat\n").await.unwrap();
            writer.write_all(b"***************************************************\n\n\n").await.unwrap();
            writer.flush().await.unwrap();

            // Notify others of join with new format
            let join_msg = format!("\n\n*** {} has joined at {} ***\n\n", 
                user.name,
                Local::now().format("%Y-%m-%d %H:%M:%S")
            );
            tx.send(join_msg).unwrap();

            loop {
                writer.write_all(b"> ").await.unwrap();
                writer.flush().await.unwrap();
                tokio::select! {
                    result = reader.read_line(&mut line) => {
                        if result.unwrap() == 0 { break; }
                        let content = line.trim().to_string();

                        if content == "exit" {
                            // Notify others of leave with new format
                            let leave_msg = format!("\n\n*** {} has left at {} ***\n\n", 
                                user.name,
                                Local::now().format("%Y-%m-%d %H:%M:%S")
                            );
                            chat_manager.remove_user(&addr);
                            tx.send(leave_msg).unwrap();
                            break;
                        } else if content.starts_with("/search ") {
                            let query = content.splitn(2, " ").nth(1).unwrap();
                            let search_results = chat_manager.search_messages_by_keyword(query);
                            if !search_results.is_empty() {
                                writer.write_all(b"Search results by keyword:\n").await.unwrap();
                                for result in search_results {
                                    writer.write_all(result.as_bytes()).await.unwrap();
                                    writer.write_all(b"\n").await.unwrap();
                                }
                            } else {
                                writer.write_all(b"No results found.\n").await.unwrap();
                            }
                            writer.flush().await.unwrap();
                        } else if content.starts_with("/user ") {
                            let query = content.splitn(2, " ").nth(1).unwrap();
                            let search_results = chat_manager.search_messages_by_user(query);
                            if !search_results.is_empty() {
                                writer.write_all(b"Search results by user:\n").await.unwrap();
                                for result in search_results {
                                    writer.write_all(result.as_bytes()).await.unwrap();
                                    writer.write_all(b"\n").await.unwrap();
                                }
                            } else {
                                writer.write_all(b"No results found.\n").await.unwrap();
                            }
                            writer.flush().await.unwrap();
                        } else {
                            let msg = Message {
                                sender_id: user.name.clone(),
                                content: content.clone(),
                                timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                            };
                            chat_manager.store_message(msg.clone());
                            tx.send(msg.format()).unwrap();
                        }

                        line.clear();
                    }
                    result = rx.recv() => {
                        if let Ok(msg) = result {
                            writer.write_all(msg.as_bytes()).await.unwrap();
                            writer.write_all(b"\n").await.unwrap();
                            writer.flush().await.unwrap();
                        }
                    }
                }
            }

            chat_manager.remove_user(&addr);
            tx.send(format!("{} has left.", user.name)).unwrap();
        });
    }
}

// To run a client, you can use `telnet 127.0.0.1 8080` in the terminal.
// For Go version, a separate implementation would be created using goroutines and net package.
