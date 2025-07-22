# PUNG
Pung is a lightweight intranet chat tool for the command line, operating entirely over UDP. The name is a nod to “ping,” a Caesar cipher of “chat,” and an acronym for Peer-to-peer UDP Network Gossip—or Grapevine, depending on your peer.


## How to use
### One liner
```bash
bash <(curl -s https://raw.githubusercontent.com/ktlast/pung/master/get-pung.sh)
```

- You can read the script [here](https://github.com/ktlast/pung/blob/master/get-pung.sh).
- Or read description below.

<br>

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

## How it works

### Steps

1. Allocate arguments and UDP socket
2. Start listeners for receiving chat, discovery, and heartbeat messages.
3. Broadcast to the common receive ports, try to find other peers.
4. Once found, record their addresses.
5. Start a UI thread for user input and send messages to each peer.

Events:
- If received a chat message, display it in the UI.
- If received a discovery message, add the peer to the peer list, then respond to the new peer.
- If received a heartbeat message, update the peer's last seen time.
