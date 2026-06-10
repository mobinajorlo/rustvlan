import express from "express";
import http from "http";
import path from "path";
import { fileURLToPath } from "url";
import { createServer as createViteServer } from "vite";
import { WebSocketServer, WebSocket } from "ws";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

interface ClientInfo {
  id: string;
  ws: WebSocket;
  username: string;
  virtualIp: string;
  roomId: string;
  joinedAt: number;
  hostedGame?: {
    game: string;
    port: number;
  } | null;
}

interface Room {
  id: string;
  name: string;
  subnet: string; // e.g., "10.8.0.0"
  clients: Map<string, ClientInfo>;
  usedIps: Set<string>;
}

const app = express();
const server = http.createServer(app);
const wss = new WebSocketServer({ noServer: true });

const PORT = 3000;

// In-memory state for virtual rooms
const rooms = new Map<string, Room>();

// Helper to assign a unique virtual IP in the 10.8.0.X subnet
function getNextVirtualIp(room: Room): string {
  for (let i = 2; i < 254; i++) {
    const ip = `10.8.0.${i}`;
    if (!room.usedIps.has(ip)) {
      room.usedIps.add(ip);
      return ip;
    }
  }
  throw new Error("No available IP addresses in this room subnet");
}

app.use(express.json());

// API: Get status of the server & rooms
app.get("/api/status", (req, res) => {
  const activeRooms = Array.from(rooms.values()).map((room) => ({
    id: room.id,
    name: room.name,
    subnet: room.subnet,
    clientCount: room.clients.size,
    clients: Array.from(room.clients.values()).map((c) => ({
      id: c.id,
      username: c.username,
      virtualIp: c.virtualIp,
    })),
  }));
  res.json({
    status: "online",
    roomsCount: rooms.size,
    activeRooms,
  });
});

// Upgrade HTTP to WS on the same port
server.on("upgrade", (request, socket, head) => {
  wss.handleUpgrade(request, socket, head, (ws) => {
    wss.emit("connection", ws, request);
  });
});

// WebSocket Server Handler
wss.on("connection", (ws) => {
  let clientSession: ClientInfo | null = null;

  ws.on("message", (rawMessage) => {
    try {
      const data = JSON.parse(rawMessage.toString());
      const { type, payload } = data;

      switch (type) {
        case "scan_rooms": {
          const activeRooms = Array.from(rooms.values()).map((room) => ({
            id: room.id,
            name: room.name,
            subnet: room.subnet,
            clientCount: room.clients.size,
            clients: Array.from(room.clients.values()).map((c) => ({
              id: c.id,
              username: c.username,
              virtualIp: c.virtualIp,
            })),
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
            // Allocate a new room
            room = {
              id: roomId,
              name: `Room - ${roomId}`,
              subnet: "10.8.0.0",
              clients: new Map(),
              usedIps: new Set(),
            };
            rooms.set(roomId, room);
          }

          if (room.clients.size >= 10) {
            ws.send(JSON.stringify({ type: "error", payload: { message: "Room is full (max 10 clients)." } }));
            return;
          }

          // Generate unique client ID & Virtual IP
          const clientId = Math.random().toString(36).substring(2, 9);
          let assignedIp: string;
          try {
            assignedIp = getNextVirtualIp(room);
          } catch (err: any) {
            ws.send(JSON.stringify({ type: "error", payload: { message: err.message } }));
            return;
          }

          clientSession = {
            id: clientId,
            ws,
            username,
            virtualIp: assignedIp,
            roomId,
            joinedAt: Date.now(),
          };

          room.clients.set(clientId, clientSession);

          // Confirm join to the client
          ws.send(
            JSON.stringify({
              type: "room_joined",
              payload: {
                id: clientId,
                roomId,
                virtualIp: assignedIp,
                peers: Array.from(room.clients.values())
                  .filter((c) => c.id !== clientId)
                  .map((c) => ({
                    id: c.id,
                    username: c.username,
                    virtualIp: c.virtualIp,
                    hostedGame: c.hostedGame,
                  })),
              },
            })
          );

          // Notify existing peers
          broadcastToRoom(roomId, clientId, {
            type: "peer_joined",
            payload: {
              id: clientId,
              username,
              virtualIp: assignedIp,
            },
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
              timestamp: Date.now(),
            },
          });
          break;
        }

        // WebRTC Signaling proxy (crucial for P2P direct transfer and low latency tunnels!)
        case "webrtc_signal": {
          if (!clientSession) return;
          const { targetId, signal } = payload;
          const room = rooms.get(clientSession.roomId);
          if (room) {
            const targetClient = room.clients.get(targetId);
            if (targetClient) {
              targetClient.ws.send(
                JSON.stringify({
                  type: "webrtc_signal",
                  payload: {
                    senderId: clientSession.id,
                    signal,
                  },
                })
              );
            }
          }
          break;
        }

        // Broadcasted network packet (UDP broadcast simulation, discovery SSDP/mDNS, Minecraft)
        case "network_broadcast_packet": {
          if (!clientSession) return;

          // Mirror this packet to ALL connected clients in the room to simulate TAP/TUN link broadcast routing
          broadcastToRoom(clientSession.roomId, null, {
            type: "network_packet_received",
            payload: {
              senderId: clientSession.id,
              senderIp: clientSession.virtualIp,
              senderName: clientSession.username,
              ...payload,
              timestamp: Date.now(),
            },
          });

          // Intercept manually announced game server mappings to dynamic memory
          if (payload.description === "GAME_SERVER_ANNOUNCEMENT(HOST)") {
            clientSession.hostedGame = {
              game: payload.protocol,
              port: Number(payload.port),
            };
          } else if (payload.description === "GAME_SERVER_ANNOUNCEMENT(STOP)") {
            clientSession.hostedGame = null;
          }
          break;
        }

        // File transfer metadata & broker
        case "file_meta": {
          if (!clientSession) return;
          const { fileName, fileSize, fileType, hash } = payload;
          broadcastToRoom(clientSession.roomId, clientSession.id, {
            type: "file_announced",
            payload: {
              senderId: clientSession.id,
              senderName: clientSession.username,
              fileName,
              fileSize,
              fileType,
              hash,
              timestamp: Date.now(),
            },
          });
          break;
        }

        case "ping": {
          ws.send(JSON.stringify({ type: "pong", payload: { timestamp: payload?.timestamp || Date.now() } }));
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

        default:
          console.warn("Unknown message type:", type);
      }
    } catch (e) {
      console.error("Failed to parse message:", e);
    }
  });

  ws.on("close", () => {
    if (clientSession) {
      const { id, roomId, virtualIp } = clientSession;
      const room = rooms.get(roomId);
      if (room) {
        room.clients.delete(id);
        room.usedIps.delete(virtualIp);

        // Notify remaining peers
        broadcastToRoom(roomId, null, {
          type: "peer_left",
          payload: {
            id,
            username: clientSession.username,
            virtualIp,
          },
        });

        // Clean up empty room
        if (room.clients.size === 0) {
          rooms.delete(roomId);
        }
      }
    }
  });
});

// Helper: Broadcast to all room members (excluding the sender if targetSenderId is supplied)
function broadcastToRoom(roomId: string, excludeClientId: string | null, message: any) {
  const room = rooms.get(roomId);
  if (!room) return;

  const serializedMsg = JSON.stringify(message);
  room.clients.forEach((client) => {
    if (excludeClientId && client.id === excludeClientId) return;
    if (client.ws.readyState === WebSocket.OPEN) {
      client.ws.send(serializedMsg);
    }
  });
}

// Vite static assets flow integration
async function bootUp() {
  if (process.env.NODE_ENV !== "production") {
    const vite = await createViteServer({
      server: { middlewareMode: true },
      appType: "spa",
    });
    app.use(vite.middlewares);
  } else {
    const distPath = path.join(process.cwd(), "dist");
    app.use(express.static(distPath));
    app.get("*", (req, res) => {
      res.sendFile(path.join(distPath, "index.html"));
    });
  }

  server.listen(PORT, "0.0.0.0", () => {
    console.log(`[VIRTUAL-NET] Coordinator server online on port ${PORT}`);
  });
}

bootUp();
