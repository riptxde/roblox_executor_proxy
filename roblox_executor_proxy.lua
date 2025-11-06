--[[
    Universal Roblox Executor Proxy Client
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
local ws = nil

-- Services
local HttpService = game:GetService("HttpService")

-- Functions
local function log(message)
    print("[Executor Proxy] " .. message)
end

local function executeMessages()
    ws.OnMessage:Connect(function(message)
        local data = HttpService:JSONDecode(message)

        if data.type == "ping" then
            -- Keep-alive mechanism
            ws:Send(HttpService:JSONEncode({type = "pong"}))
        elseif data.type == "execute" then
            local loaded, func = loadstring(data.script)

            if not loaded then
                -- Syntax error - throw error
                error(func, 0)
            else
                -- Execute and propagate runtime errors
                local success, err = pcall(func)
                if not success then
                    error(err, 0)
                end
            end
        end
    end)

    ws.OnClose:Wait()
end

-- Main
repeat
    local success, _ = pcall(function()
        ws = WebSocket.connect(url)
    end)

    if not success then
        log("Failed to connect to server at " .. url)
        log(("Attempting to reconnect in %d seconds"):format(RECONNECT_DELAY))
        wait(RECONNECT_DELAY)
        continue
    end

    log("Connected to server at " .. url)

    executeMessages()

    ws = nil
    log("Disconnected from server at " .. url)
    log(("Attempting to reconnect in %d seconds"):format(RECONNECT_DELAY))
    wait(RECONNECT_DELAY)
until nil
