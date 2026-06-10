/**
 * Standalone RustNet Virtual LAN Coordinator Server
 * 
 * To run this on your VPS:
 * 1. Ensure Node.js is installed
 * 2. Install the ws dependency: `npm install ws`
 * 3. Run: `node standalone-server.js` (Optional: specify port, e.g. `PORT=8080 node standalone-server.js`)
 */

const http = require("http");
const { WebSocketServer, WebSocket } = require("ws");

const PORT = process.env.PORT || 3000;

// Memory storage for Rooms and connected Clients
const rooms = new Map();

// Generate a random ID
function generateId() {
  return Math.random().toString(36).substring(2, 9);
}

// Assign unique virtual static IP in the 10.8.0.0/24 subnet (range 2-254)
function getNextVirtualIp(room) {
  for (let i = 2; i < 254; i++) {
    const ip = `10.8.0.${i}`;
    if (!room.usedIps.has(ip)) {
      room.usedIps.add(ip);
      return ip;
    }
  }
  throw new Error("No IP addresses available in this room subnet");
}

// Create HTTP server
const server = http.createServer((req, res) => {
  // CORS configurations
  res.setHeader("Access-Control-Allow-Origin", "*");
  res.setHeader("Access-Control-Allow-Methods", "GET, OPTIONS");
  res.setHeader("Access-Control-Allow-Headers", "Content-Type");

  if (req.method === "OPTIONS") {
    res.writeHead(204);
    res.end();
    return;
  }

  // API Status Route (used for scanning for existing rooms inside the client)
  if (req.url === "/api/status" || req.url === "/status") {
    const activeRooms = Array.from(rooms.values()).map(room => ({
      id: room.id,
      name: room.name,
      subnet: room.subnet,
      clientCount: room.clients.size,
      clients: Array.from(room.clients.values()).map(c => ({
        id: c.id,
        username: c.username,
        virtualIp: c.virtualIp
      }))
    }));

    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(JSON.stringify({
      status: "online",
      roomsCount: rooms.size,
      activeRooms
    }, null, 2));
    return;
  }

  res.writeHead(404, { "Content-Type": "text/plain" });
  res.end("RustNet Coordinator server is running. Connect via WebSockets.");
});

// Create WebSocket server on top of HTTP server
const wss = new WebSocketServer({ server });

wss.on("connection", (ws) => {
  let clientSession = null;

  ws.on("message", (rawMessage) => {
    try {
      const data = JSON.parse(rawMessage.toString());
      const { type, payload } = data;

      switch (type) {
        case "scan_rooms": {
          const activeRooms = Array.from(rooms.values()).map(room => ({
            id: room.id,
            name: room.name,
            subnet: room.subnet,
            clientCount: room.clients.size,
            clients: Array.from(room.clients.values()).map(c => ({
              id: c.id,
              username: c.username,
              virtualIp: c.virtualIp
            }))
          }));
          ws.send(JSON.stringify({
            type: "scanned_rooms",
            payload: { activeRooms }
          }));
          break;
        }

        case "join_room": {
          const { roomId, username } = payload;
          if (!roomId || !username) {
            ws.send(JSON.stringify({ type: "error", payload: { message: "Room ID and Username are required." } }));
            return;
          }

          let room = rooms.get(roomId);
          if (!room) {
            room = {
              id: roomId,
              name: `Room - ${roomId}`,
              subnet: "10.8.0.0",
              clients: new Map(),
              usedIps: new Set()
            };
            rooms.set(roomId, room);
          }

          if (room.clients.size >= 10) {
            ws.send(JSON.stringify({ type: "error", payload: { message: "Room is full (max 10)." } }));
            return;
          }

          const clientId = generateId();
          let assignedIp;
          try {
            assignedIp = getNextVirtualIp(room);
          } catch (e) {
            ws.send(JSON.stringify({ type: "error", payload: { message: e.message } }));
            return;
          }

          clientSession = {
            id: clientId,
            ws,
            username,
            virtualIp: assignedIp,
            roomId,
            joinedAt: Date.now()
          };

          room.clients.set(clientId, clientSession);

          // Confirm join to client
          ws.send(JSON.stringify({
            type: "room_joined",
            payload: {
              id: clientId,
              roomId,
              virtualIp: assignedIp,
              peers: Array.from(room.clients.values())
                .filter(c => c.id !== clientId)
                .map(c => ({
                  id: c.id,
                  username: c.username,
                  virtualIp: c.virtualIp
                }))
            }
          }));

          // Notify existing peers
          broadcastToRoom(roomId, clientId, {
            type: "peer_joined",
            payload: {
              id: clientId,
              username,
              virtualIp: assignedIp
            }
          });
          break;
        }

        case "chat_message": {
          if (!clientSession) return;
          const { message } = payload;
          broadcastToRoom(clientSession.roomId, null, {
            type: "chat_message",
            payload: {
              senderId: clientSession.id,
              senderName: clientSession.username,
              message,
              timestamp: Date.now()
            }
          });
          break;
        }

        case "ping": {
          ws.send(JSON.stringify({ type: "pong", payload: { timestamp: (payload && payload.timestamp) || Date.now() } }));
          break;
        }

        case "direct_message": {
          if (!clientSession) return;
          const { targetId, msgType, messagePayload } = payload;
          const room = rooms.get(clientSession.roomId);
          if (room) {
            const targetClient = room.clients.get(targetId);
            if (targetClient && targetClient.ws.readyState === WebSocket.OPEN) {
              targetClient.ws.send(JSON.stringify({
                type: "direct_message",
                payload: {
                  senderId: clientSession.id,
                  senderName: clientSession.username,
                  msgType,
                  messagePayload
                }
              }));
            }
          }
          break;
        }

        case "network_broadcast_packet": {
          if (!clientSession) return;
          
          broadcastToRoom(clientSession.roomId, clientSession.id, {
            type: "network_packet_received",
            payload: {
              senderId: clientSession.id,
              senderIp: clientSession.virtualIp,
              senderName: clientSession.username,
              ...payload,
              timestamp: Date.now()
            }
          });
          break;
        }

        default:
          break;
      }
    } catch (e) {
      console.error(e);
    }
  });

  ws.on("close", () => {
    if (clientSession) {
      const { id, roomId, virtualIp, username } = clientSession;
      const room = rooms.get(roomId);
      if (room) {
        room.clients.delete(id);
        room.usedIps.delete(virtualIp);

        // Notify remaining peers
        broadcastToRoom(roomId, null, {
          type: "peer_left",
          payload: {
            id,
            username,
            virtualIp
          }
        });

        // Delete empty room
        if (room.clients.size === 0) {
          rooms.delete(roomId);
        }
      }
    }
  });
});

// Broadcast helper
function broadcastToRoom(roomId, excludeClientId, message) {
  const room = rooms.get(roomId);
  if (!room) return;

  const serialized = JSON.stringify(message);
  room.clients.forEach(client => {
    if (excludeClientId && client.id === excludeClientId) return;
    if (client.ws.readyState === WebSocket.OPEN) {
      client.ws.send(serialized);
    }
  });
}

server.listen(PORT, "0.0.0.0", () => {
  console.log(`\n======================================================`);
  console.log(` RustNet Coordinator Standalone Server is Running!`);
  console.log(` Port: ${PORT}`);
  console.log(` API Status: http://localhost:${PORT}/api/status`);
  console.log(` WebSocket URL: ws://localhost:${PORT}`);
  console.log(`======================================================\n`);
});
