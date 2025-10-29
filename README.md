# Roblox Executor Proxy

A lightweight HTTP-to-WebSocket proxy server written in Rust, designed to enable external execution of Roblox scripts via any Roblox executor that supports WebSocket connections.

## Overview

This proxy provides a simple HTTP API that broadcasts scripts to connected clients, making it easy to integrate Roblox script execution with text editors or custom tooling.

*Roblox Executor Proxy* was created due to the lack of solutions for executing Roblox scripts externally. While executors like Zenith provide extensions for VSCode, support for other text editors has not been implemented. Additionally, most executors do not provide a way to execute scripts externally at all.

## Features

- **Universal Support** - Works with any platform that can make HTTP requests or run shell commands
- **Simple HTTP API** - Send file paths via POST request, proxy handles the rest
- **Fast & Lightweight** - Written in Rust with minimal resource usage
- **Zero Runtime Dependencies** - Single executable, no installation required

## Download

Get the latest release via the **Releases page**: [https://github.com/riptxde/roblox_executor_proxy/releases/](https://github.com/riptxde/roblox_executor_proxy/releases/)

## Usage

### Server Setup

1. **Run the proxy server:**
   ```bash
   roblox_executor_proxy.exe
   ```

2. **Execute scripts** by sending HTTP POST requests with the file path:
   ```bash
   curl -X POST http://localhost:13377/execute -d "C:\path\to\script.lua"
   ```

3. **Check server status:**
   ```bash
   curl http://localhost:13377/status
   ```

### Client Setup (Executor Side)

1. **Auto-execute the client script** (`roblox_executor_proxy.lua`) in your Roblox executor by copying it to your executor's designated auto-execute folder
2. The client will automatically connect to the proxy server at `ws://localhost:13378` upon joining a game
3. Once connected, all scripts sent via the HTTP API will execute automatically

## Requirements

### Server Requirements
- **Windows 10+** (The *Roblox Executor Proxy Server* requires Windows 10 or later, and was tested on the latest version of Windows 11)
- No additional dependencies required - single executable

### Executor Requirements
Your Roblox executor must support the following functions:

- **`WebSocket.connect(url)`** - Connect to WebSocket servers
- **`WebSocket.OnMessage`** - Receive messages from the server
- **`WebSocket.OnClose`** - Detect connection closures
- **`loadstring(script)()`** - Execute Lua code from strings
- **Auto-execute support** - Ability to run scripts automatically on game join

**Compatible Executors:** Most modern executors support these features, including Synapse X, Script-Ware, Krnl, Fluxus, and others with WebSocket support.

### Command-Line Options

```bash
roblox_executor_proxy [--host HOST] [--http-port PORT] [--ws-port PORT]
```

- `--host` - Server host for both HTTP and WebSocket (default: `localhost`)
- `--http-port` - HTTP server port (default: `13377`)
- `--ws-port` - WebSocket server port (default: `13378`)

**Example:**
```bash
roblox_executor_proxy --host 0.0.0.0 --http-port 8080 --ws-port 8081
```

## Editor Integration Examples

### Visual Studio Code

Install the **REST Client** extension and create a `.http` file:

```http
POST http://localhost:13377/execute
Content-Type: text/plain

C:\path\to\script.lua
```

Or use a **tasks.json** configuration:

```json
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "Execute in Roblox",
      "type": "shell",
      "command": "curl",
      "args": [
        "-X", "POST",
        "http://localhost:13377/execute",
        "-d", "${file}"
      ],
      "group": "test"
    }
  ]
}
```

Then use the [Keyboard Shortcuts Editor](https://code.visualstudio.com/docs/configure/keybindings#_keyboard-shortcuts-editor) to bind the task or REST command to a key.

### Zed Editor

Add the following to your **`tasks.json`**:

```json
{
  "label": "Roblox: Execute",
  "command": "curl -X POST http://localhost:13377/execute -d \"$ZED_FILE\""
}
```

Then bind it to a key, for example `Alt+R`, in **`keymap.json`**:

```json
{
  "context": "Editor && (extension == lua || extension == luau)",
  "bindings": {
    "alt-r": [
      "task::Spawn",
      {
        "task_name": "Roblox: Execute"
      }
    ]
  }
}
```

### Sublime Text

Add a build system (`Tools > Build System > New Build System`):

```json
{
  "shell_cmd": "curl -X POST http://localhost:13377/execute -d \"$file\"",
  "selector": "source.lua"
}
```

### Neovim

Add a keybinding in your config:

```lua
vim.keymap.set('n', '<leader>r', function()
  local file = vim.fn.expand('%:p')
  vim.fn.system('curl -X POST http://localhost:13377/execute -d "' .. file .. '"')
  print('Executed in Roblox')
end, { desc = 'Execute in Roblox' })
```

### Custom Scripts

Any language that can make HTTP requests works:

**Python:**
```python
import requests
requests.post('http://localhost:13377/execute', data=r'C:\script.lua')
```

**PowerShell:**
```powershell
Invoke-WebRequest -Uri http://localhost:13377/execute -Method POST -Body "C:\script.lua"
```

**Node.js:**
```javascript
const axios = require('axios');
axios.post('http://localhost:13377/execute', 'C:\\script.lua');
```

## API Reference

### `POST /execute`

Executes a script file in all connected Roblox executor clients.

**Request:**
- **Method:** `POST`
- **Content-Type:** `text/plain`
- **Body:** Absolute file path (e.g., `C:\Users\You\script.lua`)

**Response:**
- **200 OK** - `[SUCCESS] filename.lua sent to N client(s)`
- **207 Multi-Status** - `[PARTIAL] filename.lua sent to N/M client(s)` (some clients failed)
- **400 Bad Request** - File not found, invalid extension, or missing path
- **500 Internal Server Error** - File read error
- **503 Service Unavailable** - No clients connected

**Supported Extensions:** `.lua`, `.luau`, `.txt`

### `GET /status`

Returns the current server status and connected client count.

**Response:**
```json
{
  "status": "running",
  "connected_clients": 2,
  "timestamp": "2025-10-28T12:34:56.789Z"
}
```

## Client Script Configuration

Edit `roblox_executor_proxy.lua` to customize connection settings:

```lua
local WS_HOST = "localhost"
local WS_PORT = 13378
local RECONNECT_DELAY = 5  -- seconds
```

## Message Protocol

The server sends JSON messages to clients in this format:

```json
{
  "type": "execute",
  "script": "print('Hello from proxy!')",
  "filename": "test.lua",
  "timestamp": "2025-10-28T12:34:56.789Z"
}
```

## Building from Source

**Prerequisites:**
- [Rust](https://rustup.rs/) (1.70+)

**Build:**
```bash
git clone https://github.com/riptxde/roblox_executor_proxy.git
cd roblox_executor_proxy
cargo build --release
```

The binary will be at `target/release/roblox_executor_proxy.exe`

## Configuration

The proxy uses these default settings:

| Setting | Default | Environment |
|---------|---------|-------------|
| HTTP Host | `localhost` | `--host` flag |
| HTTP Port | `13377` | `--http-port` flag |
| WebSocket Host | `localhost` | `--host` flag |
| WebSocket Port | `13378` | `--ws-port` flag |
| Allowed Extensions | `.lua`, `.luau`, `.txt` | Hardcoded |
| Client Reconnect Interval | 5 seconds | Lua client script |

## Troubleshooting

**"No clients connected"**
- Ensure you've auto-executed `roblox_executor_proxy.lua` in your Roblox executor
- Check the executor's console for connection messages
- Verify the WebSocket port (13378) is not blocked by firewall

**"File does not exist"**
- Make sure you're sending the absolute path
- Check file path escaping in your shell/editor
- Ensure the file has a valid extension (`.lua`, `.luau`, or `.txt`)

**Client keeps disconnecting**
- Check if your executor supports persistent WebSocket connections
- Verify firewall settings aren't blocking the WebSocket port
- Check the executor's console for error messages

**"Connection refused"**
- Ensure the proxy server is running
- Verify the ports match between server and client
- Check if another program is using ports 13377 or 13378

**Script executes but nothing happens**
- Check the Roblox output/console for script errors
- Verify your script is compatible with your executor
- Test with a simple script like `print('Test')` first

## How It Works

1. **Server starts** and listens on two ports:
   - HTTP server (default: 13377) for receiving script execution requests
   - WebSocket server (default: 13378) for maintaining executor connections

2. **Executor clients** connect via WebSocket using the auto-execute Lua script

3. **When you send a script** via HTTP POST:
   - Server validates the file path and extension
   - Reads the script contents
   - Broadcasts to all connected executor clients via WebSocket
   - Each client receives and executes the script

4. **Automatic reconnection** - If connection drops, clients automatically reconnect after 5 seconds

## Security Notes

- This proxy is designed for **local development only**
- Do not expose the server to the internet without proper authentication
- Only use with trusted scripts from trusted sources
- The proxy executes scripts without sandboxing - use caution

## License

MIT License - see [LICENSE](LICENSE) for details

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
