**PeerConnect**

PeerConnect is a terminal application designed to enable seamless peer-to-peer communication. Users can register their network addresses with a central server, allowing others to retrieve these addresses and establish direct connections without the need for continuous server involvement.

### Key Features:

- **User Registration:** Users can securely register their network addresses (socket addresses) on the server, making them discoverable by other PeerConnect users.
- **Address Retrieval:** When a user wants to connect with another user, they can request the registered address from the server, facilitating direct peer-to-peer connections.
- **Decentralized Communication:** The platform ensures that once the addresses are exchanged, users communicate directly, eliminating the server as an intermediary.
- **Rust-based Implementation:** Built with Rust, ensuring high performance, reliability, and security.
- **Asynchronous Operations with Tokio:** Utilizes Tokio for efficient asynchronous handling of multiple connections, enhancing responsiveness and scalability.
- **Uses Web Socket Protocol: **it uses web socket protocols to establish the connection between peers. So, there is no latency delay over the communicating the messages.

### Benefits:

- **Reduced Server Load:** By enabling direct peer-to-peer connections, the server is relieved from handling continuous message traffic, focusing only on address management.
- **Enhanced Privacy:** Direct user-to-user connections minimize the risk of data interception, boosting privacy and security.
- **Scalability:** The platform's architecture supports scalability as the server's role is limited, allowing it to handle a large number of users efficiently.
- **Performance:** Leveraging Rust's safety and performance features along with Tokio's asynchronous capabilities ensures a robust and high-performing communication system.

PeerConnect transforms user communication by providing a decentralized, efficient, and secure way to establish direct peer-to-peer connections.

* * *

# How to use ?

1.   There mainly two files in  the bin folder **server.rs** and **peer.rs.**
2.  cargo run --bin server runs the server on the port 8080.
3.  And then run cargo run --bin peer to run the peer on other port give port number as input to bind the port.
4.  select the service you want to see.
5.  exchange the messages.
