# PUNG
Pung is a lightweight intranet chat tool for the command line, operating entirely over UDP. The name is a nod to “ping,” a Caesar cipher of “chat,” and an acronym for Peer-to-peer UDP Network Gossip—or Grapevine, depending on your peer.


## How to use

### For MacOS
#### Option 1: GUI
1. Download a release from [GitHub](https://github.com/ktlast/pung/releases).
2. Extract the tar.gz file.
3. Open terminal and navigate to the extracted directory. You can try `./pung -u my_name` to see if it works.
4. If it doesn't work, open System Settings -> Security & Privacy -> General -> Allow apps downloaded from ...
5. Run `./pung -u my_name` again

#### Option 2: Pure Command
You may need to remove the quarantine attribute after downloading:
```bash
# Check if jq is installed, since we need it to parse the latest version.
command -v jq >/dev/null 2>&1 || { echo >&2 "Please install jq first."; exit 1; }

# Get the latest version from GitHub API
version=$(curl -s https://api.github.com/repos/ktlast/pung/releases/latest | jq -r '.tag_name')
full_name="pung-${version}-aarch64-apple-darwin"

# Download the latest release
download_url="https://github.com/ktlast/pung/releases/download/${version}/${full_name}.tar.gz"
curl -L ${download_url} -o ${full_name}.tar.gz
mkdir -p ${full_name} \
    && tar -xzf ${full_name}.tar.gz -C ${full_name} \
    && cd ${full_name}

# Remove quarantine attribute
sudo xattr -d com.apple.quarantine ./pung

# Start the app
./pung -u my_name
```


### For Linux

```bash
# check if jq is installed
command -v jq >/dev/null 2>&1 || { echo >&2 "Please install jq first."; exit 1; }

# Get the latest version from GitHub API
version=$(curl -s https://api.github.com/repos/ktlast/pung/releases/latest | jq -r '.tag_name')
full_name="pung-${version}-x86_64-unknown-linux-gnu"

# Download the latest release
download_url="https://github.com/ktlast/pung/releases/download/${version}/${full_name}.tar.gz"
curl -L ${download_url} -o ${full_name}.tar.gz
mkdir -p ${full_name} \
    && tar -xzf ${full_name}.tar.gz -C ${full_name} \
    && cd ${full_name}

# Start the app
./pung -u my_name
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