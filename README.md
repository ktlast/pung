# rossip
 A simple intranet chat application designed to run entirely in the command line (CLI/terminal).

<br>

## Notes
1. 2025/04: Just start learning Rust lang, so in the early stage, the code may be not so good.


<br>

## folder plans (ChatGPT)

- file structure
```
rossip/
├── Cargo.toml
├── src/
│   ├── main.rs               # entry point, sets up the async runtime
│   ├── config.rs             # config structs, constants, CLI args if needed
│   ├── message.rs            # message format + (de)serialization, message ID logic
│   ├── peer/
│   │   ├── mod.rs            # peer list manager, discovery, heartbeat
│   │   ├── discovery.rs      # mDNS or UDP "hello" logic
│   │   ├── tracker.rs        # heartbeat, timeouts, peer liveness
│   ├── net/
│   │   ├── mod.rs            # UDP socket setup
│   │   ├── broadcaster.rs    # sending messages to peers
│   │   ├── listener.rs       # receives + parses incoming messages
│   ├── ui/
│   │   ├── mod.rs            # user I/O manager (CLI input/output)
│   │   ├── input.rs          # reads user input from terminal
│   │   ├── output.rs         # displays messages to terminal
│   ├── utils.rs              # message ID cache, timestamp utils, etc.
```

- crates maybe needed
```
[dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
bincode = "1.3"
dashmap = "5"
uuid = { version = "1", features = ["v4"] }
socket2 = "0.5"
mdns = "1.0"  # optional, or use `async-mdns`
clap = "4"    # optional, CLI arg parsing
chrono = "0.4"  # timestamps and timeouts
```

<br>

## Appendix: Architecture Notes (Design thoughts from ChatGPT)

That’s an awesome little project to learn Rust with! You're building a decentralized, local-network-only gossip protocol, and that checks a lot of boxes for learning about systems, networking, and Rust concurrency. Let's break it down and talk concepts, protocols, and crates/tools you might consider.

### ✅ 1. Message Broadcasting

🌐 Concept:
You want local-network-only broadcasting (no internet) with no central server, where each peer can send messages that get gossiped to all others. There are two main ways you could think about this:

#### A) UDP Broadcast or Multicast
- Broadcast: Send to the special IP 255.255.255.255 or subnet-based (like 192.168.1.255). Every device on that subnet gets it.

- Multicast: More controlled than broadcast. Peers subscribe to a multicast group (like 224.0.0.1), and messages sent to that group are received by all subscribed devices.

These are both connectionless, fast, and ideal for LAN. Downside: no guarantees of delivery, ordering, etc. But for IRC-like ephemeral messages? Totally fine.

Multicast is usually a better choice if you want less noisy traffic and slightly better control. Not every environment supports it out of the box though.

#### B) Gossip Protocol (Flooding, Epidemic style)
- In true P2P style, you could have each node forward messages it receives to its known peers — like a rumor spreading. This scales better than raw UDP broadcast and can reduce redundant messages if done well (e.g., with message IDs to avoid loops).

🚀 Tools/Crates:
- tokio — for async networking

- [tokio::net::UdpSocket] or standard std::net::UdpSocket

- socket2 — more control over socket options (needed for multicast)

- mdns — useful for service discovery via multicast DNS if you want to avoid manual IP setup

### ✅ 2. Peer Discovery & Maintaining Peer List
🌐 Concept:
Since it's P2P and dynamic, each peer needs to:

1. Discover others on the network

2. Keep a list of currently alive peers

3. Maybe periodically ping or heartbeat to check who's alive

Strategies:

#### A) mDNS (Multicast DNS)
- Used by many zero-conf networking protocols (e.g., Apple’s Bonjour)

- Peers announce themselves and listen for announcements.

- Great for LAN service discovery.

Can update peer list dynamically.

#### B) Custom UDP Broadcast for Discovery
- On startup, send a "HELLO" message to the broadcast IP (or multicast group).

- All peers listening respond with their identity.

- Maintain a peer list based on who replies.

#### C) Heartbeat/Ping
- Periodically send a "I'm alive" message.

- If you haven’t heard from a peer in N seconds, remove it from the list.

#### D) Message-based Updates
Every message can carry a list of known peers (partial list), which helps in slowly syncing the peer list across the network — very gossip-like.

🔧 Tools/Crates:
- mdns

- libp2p (Rust crate: libp2p) — it's much heavier, but if you want to go deep into P2P, this has everything (gossip, DHTs, pubsub). Maybe overkill for now, but worth knowing.

- chrono or [instant] — for tracking peer timeouts

- dashmap — thread-safe peer list if you're sharing state

### ✅ 3. Design Thoughts (Architecturally)
- Message Format: Use simple structs — sender ID, timestamp, message content, maybe a message ID (to prevent re-sends/loops).

- Loop Prevention: If you gossip messages via flooding, assign unique IDs to messages and keep a small cache of seen IDs.

- Concurrency Model: Use Rust’s async features (tokio) to listen on sockets, send heartbeats, handle incoming messages all concurrently.

- Separation of Concerns: One task for message I/O, one for peer management, one for user input/output.

### ✅ 4. Summary

|Feature | Concept | Crate / Tool|
|---|---|---|
|Message Broadcast | UDP Multicast / Broadcast / Gossip Flooding | tokio, socket2
|Peer Discovery | mDNS / Custom UDP ping | mdns, tokio, chrono
|Peer Tracking | Heartbeats / Message Carrying Peers | dashmap, chrono
|Async Networking | Concurrent tasks | tokio, futures
|Optional deeper P2P | Full-fledged protocols | libp2p