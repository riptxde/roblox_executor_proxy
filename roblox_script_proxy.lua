--[[
    Universal Roblox Script Proxy Client
    Auto-execute this script to connect to the proxy server
    Receives and executes scripts sent via the HTTP API

    Configuration:
    - Change WS_HOST and WS_PORT to match your server settings
]]

-- Configuration
local WS_HOST = "localhost"
local WS_PORT = 13378
local RECONNECT_DELAY = 5

-- Globals
local wait = task.wait
local url = ("ws://%s:%d"):format(WS_HOST, WS_PORT)
local script_proxy_ws = nil

-- Services
local HttpService = game:GetService("HttpService")

-- Functions
local function log(message)
    print("[Script Proxy] " .. message)
end

local function executeMessages()
    script_proxy_ws.OnMessage:Connect(function(message)
        local data = HttpService:JSONDecode(message)

        if data.type == "execute" then
            loadstring(data.script)()
        end
    end)

    script_proxy_ws.OnClose:Wait()
end

-- Main
repeat
    local success, _ = pcall(function()
        script_proxy_ws = WebSocket.connect(url)
    end)

    if not success then
        log("Failed to connect to server at " .. url)
        log(("Attempting to reconnect in %d seconds"):format(RECONNECT_DELAY))
        wait(RECONNECT_DELAY)
        continue
    end

    log("Connected to server at " .. url)

    executeMessages()

    script_proxy_ws = nil
    log("Disconnected from server at " .. url)
    log(("Attempting to reconnect in %d seconds"):format(RECONNECT_DELAY))
    wait(RECONNECT_DELAY)
until nil
