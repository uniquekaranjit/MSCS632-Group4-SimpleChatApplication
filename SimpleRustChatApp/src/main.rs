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

    // Method to search messages by user
    fn search_messages_by_user(&self, user_name: &str) -> Vec<String> {
        self.messages.lock().unwrap().iter()
            .filter(|msg| msg.sender_id == user_name)
            .map(|m| m.format())
            .collect()
    }

    // Method to search messages by keyword
    fn search_messages_by_keyword(&self, keyword: &str) -> Vec<String> {
        self.messages.lock().unwrap().iter()
            .filter(|msg| msg.matches(keyword))
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

            writer.write_all(b"Enter your name: ").await.unwrap();
            reader.read_line(&mut line).await.unwrap();
            let name = line.trim().to_string();
            let user = chat_manager.register_user(addr, name);
            let user_id = user.id.clone();
            line.clear();

            tx.send(format!("{} has joined.", user.name)).unwrap();

            loop {
                tokio::select! {
                    result = reader.read_line(&mut line) => {
                        if result.unwrap() == 0 { break; }
                        let content = line.trim().to_string();

                        if content.starts_with("search user:") {
                            // Extract user name from the command
                            let user_name = content.splitn(2, ":").nth(1).unwrap().trim();
                            let search_results = chat_manager.search_messages_by_user(user_name);
                            let result_str = search_results.join("\n");
                            writer.write_all(result_str.as_bytes()).await.unwrap();
                            writer.write_all(b"\n").await.unwrap();
                        } else if content.starts_with("search keyword:") {
                            // Extract keyword from the command
                            let keyword = content.splitn(2, ":").nth(1).unwrap().trim();
                            let search_results = chat_manager.search_messages_by_keyword(keyword);
                            let result_str = search_results.join("\n");
                            writer.write_all(result_str.as_bytes()).await.unwrap();
                            writer.write_all(b"\n").await.unwrap();
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
// Clients can now issue the command to search by user or keyword.
