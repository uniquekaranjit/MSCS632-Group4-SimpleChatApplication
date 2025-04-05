package main

import (
	"bufio"
	"fmt"
	"net"
	"strings"
	"sync"
	"time"
)

type User struct {
	ID   string
	Name string
}

type Message struct {
	SenderID  string
	Content   string
	Timestamp string
}

type ChatManager struct {
	messages    []Message
	users       map[string]User
	mu          sync.Mutex
	clients     map[net.Conn]User
	messageChan chan Message
}

func NewChatManager() *ChatManager {
	return &ChatManager{
		messages:    make([]Message, 0),
		users:       make(map[string]User),
		clients:     make(map[net.Conn]User),
		messageChan: make(chan Message, 100),
	}
}

func (cm *ChatManager) StoreMessage(msg Message) {
	cm.mu.Lock()
	defer cm.mu.Unlock()
	cm.messages = append(cm.messages, msg)
}

func (cm *ChatManager) SearchMessages(query string, searchByUser bool) []string {
	cm.mu.Lock()
	defer cm.mu.Unlock()

	var results []string
	for _, msg := range cm.messages {
		if searchByUser {
			if strings.Contains(msg.SenderID, query) {
				results = append(results, fmt.Sprintf("[%s] %s: %s", msg.Timestamp, msg.SenderID, msg.Content))
			}
		} else {
			if strings.Contains(msg.Content, query) {
				results = append(results, fmt.Sprintf("[%s] %s: %s", msg.Timestamp, msg.SenderID, msg.Content))
			}
		}
	}
	return results
}

func (cm *ChatManager) RegisterUser(id, name string, conn net.Conn) User {
	cm.mu.Lock()
	defer cm.mu.Unlock()
	user := User{ID: id, Name: name}
	cm.users[id] = user
	cm.clients[conn] = user
	return user
}

func (cm *ChatManager) RemoveUser(conn net.Conn) {
	cm.mu.Lock()
	defer cm.mu.Unlock()
	user := cm.clients[conn]
	delete(cm.users, user.ID)
	delete(cm.clients, conn)
}

func (cm *ChatManager) BroadcastMessage(msg Message) {
	cm.mu.Lock()
	defer cm.mu.Unlock()

	// Only broadcast to clients, don't store the message here
	for conn := range cm.clients {
		fmt.Fprintf(conn, "[%s] %s: %s\n", msg.Timestamp, msg.SenderID, msg.Content)
	}
}

func handleClient(conn net.Conn, cm *ChatManager) {
	defer conn.Close()

	// Get the user's name
	fmt.Fprint(conn, "Enter your name: ")
	scanner := bufio.NewScanner(conn)
	scanner.Scan()
	name := scanner.Text()

	// Register the user
	user := cm.RegisterUser(conn.RemoteAddr().String(), name, conn)

	// Show command instructions once
	fmt.Fprintln(conn, "Commands available:")
	fmt.Fprintln(conn, "- Type 'exit' to leave")
	fmt.Fprintln(conn, "- Type '/search <query>' to search by keyword")
	fmt.Fprintln(conn, "- Type '/user <username>' to search by user")
	fmt.Fprintln(conn, "- Type any other message to chat")

	// Notify others
	msg := Message{
		SenderID:  user.Name,
		Content:   fmt.Sprintf("%s has joined.", user.Name),
		Timestamp: time.Now().Format("2006-01-02 15:04:05"),
	}
	cm.BroadcastMessage(msg)

	// Handle incoming messages from the client
	for {
		fmt.Fprint(conn, "> ")  // Simple prompt for messages
		scanner.Scan()
		message := scanner.Text()

		// Exit condition
		if message == "exit" {
			break
		}

		// Search by keyword command
		if strings.HasPrefix(message, "/search ") {
			query := strings.TrimPrefix(message, "/search ")
			results := cm.SearchMessages(query, false)

			// Send the search results only to the requesting client
			if len(results) > 0 {
				fmt.Fprintln(conn, "Search results by keyword:")
				for _, result := range results {
					fmt.Fprintln(conn, result)
				}
			} else {
				fmt.Fprintln(conn, "No results found.")
			}
			continue
		}

		// Search by user command
		if strings.HasPrefix(message, "/user ") {
			query := strings.TrimPrefix(message, "/user ")
			results := cm.SearchMessages(query, true)

			// Send the search results only to the requesting client
			if len(results) > 0 {
				fmt.Fprintln(conn, "Search results by user:")
				for _, result := range results {
					fmt.Fprintln(conn, result)
				}
			} else {
				fmt.Fprintln(conn, "No results found.")
			}
			continue
		}

		// Store and broadcast the message
		msg := Message{
			SenderID:  user.Name,
			Content:   message,
			Timestamp: time.Now().Format("2006-01-02 15:04:05"),
		}
		cm.StoreMessage(msg)  // Store the message
		cm.BroadcastMessage(msg)  // Broadcast to all clients
	}

	// Remove the user when they disconnect
	cm.RemoveUser(conn)
	msg = Message{
		SenderID:  user.Name,
		Content:   fmt.Sprintf("%s has left.", user.Name),
		Timestamp: time.Now().Format("2006-01-02 15:04:05"),
	}
	cm.BroadcastMessage(msg)
}

func main() {
	chatManager := NewChatManager()

	// Start the server
	ln, err := net.Listen("tcp", ":8080")
	if err != nil {
		fmt.Println("Error starting server:", err)
		return
	}
	defer ln.Close()

	fmt.Println("Server is listening on port 8080...")

	for {
		conn, err := ln.Accept()
		if err != nil {
			fmt.Println("Error accepting connection:", err)
			continue
		}
		go handleClient(conn, chatManager)
	}
}
