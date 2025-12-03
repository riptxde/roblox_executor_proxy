# Roblox Executor Proxy

A lightweight HTTP-to-WebSocket proxy server written in Rust, designed to enable external execution of Roblox scripts via any Roblox executor that supports WebSocket connections.

## Overview

This proxy provides a simple HTTP API that broadcasts scripts to connected clients, making it easy to integrate Roblox script execution with text editors or custom tooling.

*Roblox Executor Proxy* was created due to the lack of solutions for executing Roblox scripts externally. While executors like Zenith provide extensions for VSCode, support for other text editors has not been implemented. Additionally, most executors do not provide a way to execute scripts externally at all.

## Features

- **Universal Support** - Works with any platform that can make HTTP requests or run shell commands
- **Simple HTTP API** - Send file paths via POST request, proxy handles the rest
- **Easy Integration** - Easily integrate roblox script execution in your text editor without even needing to download extensions
- **Fast & Lightweight** - Written in Rust with minimal resource usage
- **Zero Runtime Dependencies** - Single executable, no installation required

## Usage

1. **Add the client script to your executor's auto-execute folder:**
   - Download the client script (`roblox_executor_proxy.lua`) from the [Releases page](https://github.com/riptxde/roblox_executor_proxy/releases/)
   - Copy the script to your executor's designated auto-execute folder

2. **Run the proxy server:**
   - Download the server executable (`roblox_executor_proxy.exe`) from the [Releases page](https://github.com/riptxde/roblox_executor_proxy/releases/)
   - Run the executable and ensure it remains running in the background while you execute scripts

3. **Attach to roblox:**
   - Run roblox and attach to the roblox client using your script executor
   - Ensure the client script is loaded and ready to receive requests by checking the dev console (`F9`) for the startup message

4. **Execute scripts** by sending HTTP POST requests with the file path:
   ```bash
   curl -X POST http://localhost:13377/execute_file -d "C:\path\to\script.lua"
   ```
   You can also [integrate this into your text editor](#editor-integration-examples) so you do not need to manually run terminal commands to execute scripts

## Requirements

### Server Requirements
- **Windows 10+** (The *Roblox Executor Proxy Server* requires Windows 10 or later, and was tested on the latest version of Windows 11)
- No additional dependencies required - single executable

### Executor Requirements
Your Roblox executor must support the following functions:

- **`WebSocket.connect(url)`** - Connect to WebSocket servers
- **`WebSocket.OnMessage`** - Receive messages from the server
- **`WebSocket.OnClose`** - Detect connection closures
- **`loadstring(script)`** - Execute Lua code from strings
- **Auto-execute support** - Ability to run scripts automatically on game join

## Editor Integration Examples

### Visual Studio Code

Add the following to your VSCode's `tasks.json`:

```json
{
	"label": "Roblox: Execute",
	"type": "shell",
	"command": "curl",
	"args": [
		"-X", "POST",
		"http://localhost:13377/execute_file",
		"-d", "${file}"
	]
}
```

Then, you can run the task via the command palette. You can also bind it to a key, for example `Ctrl+R`. You can do so by adding the following to your VSCode's `keybindings.json`:

```json
{
    "key": "ctrl+r",
    "command": "workbench.action.tasks.runTask",
    "when": "resourceExtname == .lua || resourceExtname == .luau",
    "args": "Roblox: Execute"
}
```

### Zed Editor

Add the following to your Zed's `tasks.json`:

```json
{
  "label": "Roblox: Execute",
  "command": "curl -X POST http://localhost:13377/execute_file -d \"$ZED_FILE\""
}
```

Then, you can run the task via the command palette. You can also bind it to a key, for example `Alt+R`. You can do so by adding the following to your Zed's `keymap.json`:

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
  "shell_cmd": "curl -X POST http://localhost:13377/execute_file -d \"$file\"",
  "file_patterns": ["*.lua", "*.luau"]
}
```

And save this as `Roblox.sublime-build` or any filename of your choice.

Then you can run a script by selecting `Tools > Build`, or by pressing `Ctrl+B`.

### Neovim

Add a keybinding in your config:

```lua
vim.keymap.set('n', '<leader>r', function()
  local file = vim.fn.expand('%:p')
  vim.fn.system('curl -X POST http://localhost:13377/execute_file -d "' .. file .. '"')
  print('Executed in Roblox')
end, { desc = 'Execute in Roblox' })
```

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

## API Reference

### `POST /execute_file`

Executes a script file in all connected Roblox executor clients.

**Request:**
- **Method:** `POST`
- **Content-Type:** `text/plain`
- **Body:** Absolute file path (e.g., `C:\Users\You\script.lua`)

**Response:**

All responses return JSON with the following structure:

```json
{
  "success": true,
  "message": "Script 'example.lua' sent to all connected clients",
  "clients_reached": 2,
  "total_clients": 2
}
```

**Status Codes:**
- **200 OK** - Script successfully sent to all clients
  ```json
  {
    "success": true,
    "message": "Script 'filename.lua' sent to all connected clients",
    "clients_reached": 2,
    "total_clients": 2
  }
  ```

- **207 Multi-Status** - Script sent to some but not all clients
  ```json
  {
    "success": false,
    "error": "Script 'filename.lua' only reached 1/2 clients",
    "clients_reached": 1,
    "total_clients": 2
  }
  ```

- **400 Bad Request** - Invalid request (file not found, wrong extension, etc.)
  ```json
  {
    "success": false,
    "error": "File 'C:\\path\\to\\script.lua' does not exist"
  }
  ```

- **500 Internal Server Error** - Server error (file read error, serialization error)
  ```json
  {
    "success": false,
    "error": "Error reading file: Permission denied"
  }
  ```

- **503 Service Unavailable** - No clients connected
  ```json
  {
    "success": false,
    "error": "No clients connected",
    "clients_reached": 0,
    "total_clients": 0
  }
  ```

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
