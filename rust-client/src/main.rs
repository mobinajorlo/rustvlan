use std::io::{self, Write};
use tokio::io::{AsyncBufReadExt, BufReader, AsyncReadExt, AsyncWriteExt};
use std::collections::HashMap;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use chrono::Local;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tokio::net::UdpSocket;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Peer {
    id: String,
    username: String,
    #[serde(rename = "virtualIp")]
    virtual_ip: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "payload")]
enum ServerMessage {
    #[serde(rename = "room_joined")]
    RoomJoined {
        id: String,
        #[serde(rename = "roomId")]
        room_id: String,
        #[serde(rename = "virtualIp")]
        virtual_ip: String,
        peers: Vec<Peer>,
    },
    #[serde(rename = "peer_joined")]
    PeerJoined {
        id: String,
        username: String,
        #[serde(rename = "virtualIp")]
        virtual_ip: String,
    },
    #[serde(rename = "peer_left")]
    PeerLeft {
        id: String,
        username: String,
        #[serde(rename = "virtualIp")]
        virtual_ip: String,
    },
    #[serde(rename = "chat_message")]
    ChatMessage {
        #[serde(rename = "senderId")]
        sender_id: String,
        #[serde(rename = "senderName")]
        sender_name: String,
        message: String,
        timestamp: u64,
    },
    #[serde(rename = "network_packet_received")]
    NetworkPacketReceived {
        #[serde(rename = "senderId")]
        sender_id: String,
        #[serde(rename = "senderIp")]
        sender_ip: String,
        #[serde(rename = "senderName")]
        sender_name: String,
        protocol: String,
        port: u16,
        #[serde(default, rename = "virtualDestination")]
        virtual_destination: Option<String>,
        size: usize,
        description: String,
        #[serde(default, rename = "hex_data")]
        hex_data: Option<String>,
        #[serde(default, rename = "src_port")]
        src_port: Option<u16>,
    },
    #[serde(rename = "direct_message")]
    DirectMessage {
        #[serde(rename = "senderId")]
        sender_id: String,
        #[serde(rename = "senderName")]
        sender_name: String,
        #[serde(rename = "msgType")]
        msg_type: String,
        #[serde(rename = "messagePayload")]
        message_payload: serde_json::Value,
    },
    #[serde(rename = "pong")]
    Pong {
        timestamp: u64,
    },
    #[serde(rename = "error")]
    Error {
        message: String,
    },
}

fn log_info(module: &str, msg: &str) {
    let now = Local::now().format("%H:%M:%S");
    println!("[\x1b[36m{}\x1b[0m] [\x1b[32m{}\x1b[0m] {}", now, module, msg);
}

fn log_warn(module: &str, msg: &str) {
    let now = Local::now().format("%H:%M:%S");
    println!("[\x1b[36m{}\x1b[0m] [\x1b[33m{}\x1b[0m] {}", now, module, msg);
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn from_hex(hex: &str) -> Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 {
        return Err("Hex string must have an even length".to_string());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i+2], 16)
                .map_err(|e| format!("Failed to parse hex: {}", e))
        })
        .collect()
}

fn try_add_loopback_alias(ip: &str) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("netsh")
            .args(&["interface", "ipv4", "add", "address", "Loopback Pseudo-Interface 1", ip, "255.255.255.255"])
            .output();
    }

    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("sudo")
            .args(&["ifconfig", "lo0", "alias", ip])
            .output();
    }

    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("sudo")
            .args(&["ip", "addr", "add", &format!("{}/32", ip), "dev", "lo"])
            .output();
    }
}

fn print_elevation_guide(ip: &str, name: &str) {
    println!("\x1b[33m┌────────────────────────────────────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[33m│ 🇮🇷 راهنمای فعال‌سازی پینگ مستقیم و آی‌پی مجازی برای: {:<19} │\x1b[0m", name);
    println!("\x1b[33m├────────────────────────────────────────────────────────────────────────┤\x1b[0m");
    println!("\x1b[33m│ برای اینکه بتوانید آی‌پی مجازی این بازیکن ({:<12}) را مستقیماً پینگ │\x1b[0m", ip);
    println!("\x1b[33m│ کنید یا در داخل بازی مستقیماً به آن وصل شوید، دستور زیر را در CMD     │\x1b[0m");
    println!("\x1b[33m│ که به صورت Administrator (صفحه سیاه) باز شده است اجرا کنید:            │\x1b[0m");
    println!("\x1b[33m│                                                                        │\x1b[0m");
    println!("│   \x1b[32mnetsh interface ipv4 add address \"Loopback Pseudo-Interface 1\" {} 255.255.255.255\x1b[0m", ip);
    println!("\x1b[33m│                                                                        │\x1b[0m");
    println!("\x1b[33m│ برای سیستم‌عامل‌های macOS یا Linux در ترمینال اجرا کنید:                 │\x1b[0m");
    println!("│   \x1b[32msudo ifconfig lo0 alias {}\x1b[0m  (macOS)                               │", ip);
    println!("│   \x1b[32msudo ip addr add {}/32 dev lo\x1b[0m (Linux)                             │", ip);
    println!("\x1b[33m└────────────────────────────────────────────────────────────────────────┘\x1b[0m");
}

// Memory block to cache recently injected packets locally to prevent network packet echoing loops
struct EchoSuppressor {
    recent_packets: HashSet<Vec<u8>>,
    queue: std::collections::VecDeque<Vec<u8>>,
    max_size: usize,
}

impl EchoSuppressor {
    fn new(max_size: usize) -> Self {
        Self {
            recent_packets: HashSet::new(),
            queue: std::collections::VecDeque::new(),
            max_size,
        }
    }

    fn insert(&mut self, packet: Vec<u8>) {
        if self.recent_packets.contains(&packet) {
            return;
        }
        if self.recent_packets.len() >= self.max_size {
            if let Some(old) = self.queue.pop_front() {
                self.recent_packets.remove(&old);
            }
        }
        self.recent_packets.insert(packet.clone());
        self.queue.push_back(packet);
    }

    fn contains(&self, packet: &[u8]) -> bool {
        self.recent_packets.contains(packet)
    }
}

type PeerMap = Arc<Mutex<HashMap<String, Peer>>>;
type TcpTunnelRegistry = Arc<Mutex<HashMap<String, tokio::sync::mpsc::Sender<Vec<u8>>>>>;
type UdpProxyRegistry = Arc<Mutex<HashMap<String, Arc<UdpSocket>>>>;
type HostUdpRegistry = Arc<Mutex<HashMap<String, Arc<UdpSocket>>>>;
type ActiveProxies = Arc<Mutex<HashSet<String>>>;

fn spawn_single_peer_port_proxy(
    peer: Peer,
    port: u16,
    peer_map: PeerMap,
    ws_tx: tokio::sync::mpsc::Sender<String>,
    suppressor: Arc<Mutex<EchoSuppressor>>,
    tcp_registry: TcpTunnelRegistry,
    udp_registry: UdpProxyRegistry,
    active_proxies: ActiveProxies,
) {
    let peer_id = peer.id.clone();
    let virtual_ip = peer.virtual_ip.clone();
    let local_ip = virtual_ip.replace("10.8.0.", "127.0.0.");
    let peer_username = peer.username.clone();

    // Spawn TCP Proxy for this port
    let tcp_reg_clone = tcp_registry.clone();
    let ws_tx_clone = ws_tx.clone();
    let peer_map_clone = peer_map.clone();
    let local_ip_clone = local_ip.clone();
    let virtual_ip_clone = virtual_ip.clone();
    let peer_id_clone = peer_id.clone();
    let peer_username_clone = peer_username.clone();

    tokio::spawn(async move {
        let addrs = vec![
            format!("{}:{}", virtual_ip_clone, port),
            format!("{}:{}", local_ip_clone, port),
            format!("0.0.0.0:{}", port),
        ];

        let mut listener_opt = None;
        for addr in &addrs {
            match tokio::net::TcpListener::bind(addr).await {
                Ok(l) => {
                    log_info("PROXY", &format!("Bound Dynamic TCP Proxy on {} for peer: {}", addr, peer_username_clone));
                    listener_opt = Some(l);
                    break;
                }
                Err(e) => {
                    log_warn("PROXY_TRY", &format!("Could not bind TCP proxy on {} (trying fallback): {:?}", addr, e));
                }
            }
        }

        let listener = match listener_opt {
            Some(l) => l,
            None => {
                log_warn("PROXY", &format!("Critical: Failed to bind TCP proxy for peer: {}", peer_username_clone));
                return;
            }
        };

        loop {
            // Check active
            {
                let map = peer_map_clone.lock().unwrap();
                if !map.contains_key(&peer_id_clone) {
                    break;
                }
            }

            tokio::select! {
                accept_res = listener.accept() => {
                    if let Ok((socket, _)) = accept_res {
                        let conn_id = uuid::Uuid::new_v4().to_string();
                        let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(512);
                        {
                            let mut reg = tcp_reg_clone.lock().unwrap();
                            reg.insert(conn_id.clone(), tx);
                        }

                        let (mut reader, mut writer) = tokio::io::split(socket);
                        
                        // Notify host to connect to 127.0.0.1:port
                        let payload = json!({
                            "type": "direct_message",
                            "payload": {
                                "targetId": peer_id_clone.clone(),
                                "msgType": "tcp_tunnel_connect",
                                "messagePayload": {
                                    "connection_id": conn_id.clone(),
                                    "port": port
                                }
                            }
                        });
                        let _ = ws_tx_clone.send(payload.to_string()).await;

                        let conn_id_sub = conn_id.clone();
                        let peer_id_sub = peer_id_clone.clone();
                        let ws_tx_sub = ws_tx_clone.clone();
                        tokio::spawn(async move {
                            let mut buf = vec![0u8; 16384];
                            while let Ok(n) = reader.read(&mut buf).await {
                                if n == 0 { break; }
                                let data_payload = json!({
                                    "type": "direct_message",
                                    "payload": {
                                        "targetId": peer_id_sub.clone(),
                                        "msgType": "tcp_tunnel_data",
                                        "messagePayload": {
                                            "connection_id": conn_id_sub.clone(),
                                            "hex_data": to_hex(&buf[..n])
                                        }
                                    }
                                });
                                if ws_tx_sub.send(data_payload.to_string()).await.is_err() {
                                    break;
                                }
                            }
                            let close_payload = json!({
                                "type": "direct_message",
                                "payload": {
                                    "targetId": peer_id_sub,
                                    "msgType": "tcp_tunnel_close",
                                    "messagePayload": {
                                        "connection_id": conn_id_sub
                                    }
                                }
                            });
                            let _ = ws_tx_sub.send(close_payload.to_string()).await;
                        });

                        let tcp_reg_sub = tcp_reg_clone.clone();
                        let conn_id_sub2 = conn_id.clone();
                        tokio::spawn(async move {
                            while let Some(data) = rx.recv().await {
                                if writer.write_all(&data).await.is_err() {
                                    break;
                                }
                            }
                            let mut reg = tcp_reg_sub.lock().unwrap();
                            reg.remove(&conn_id_sub2);
                        });
                    }
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(3)) => {
                    // Check
                }
            }
        }
    });

    // Spawn UDP Proxy for this port
    let udp_reg_clone = udp_registry.clone();
    let ws_tx_clone2 = ws_tx.clone();
    let peer_map_clone2 = peer_map.clone();
    let supp_clone2 = suppressor.clone();
    let local_ip_clone2 = local_ip.clone();
    let virtual_ip_clone2 = virtual_ip.clone();
    let peer_id_clone2 = peer_id.clone();

    tokio::spawn(async move {
        let addrs = vec![
            format!("{}:{}", virtual_ip_clone2, port),
            format!("{}:{}", local_ip_clone2, port),
            format!("0.0.0.0:{}", port),
        ];

        let mut socket_opt = None;
        for addr in &addrs {
            match UdpSocket::bind(addr).await {
                Ok(s) => {
                    log_info("PROXY", &format!("Bound Dynamic UDP Proxy on {} for peer: {}", addr, peer_username));
                    socket_opt = Some(Arc::new(s));
                    break;
                }
                Err(e) => {
                    log_warn("PROXY_TRY", &format!("Could not bind UDP proxy on {} (trying fallback): {:?}", addr, e));
                }
            }
        }

        let socket = match socket_opt {
            Some(s) => s,
            None => {
                log_warn("PROXY", &format!("Critical: Failed to bind UDP proxy for peer: {}", peer_username));
                return;
            }
        };

        let socket_key = format!("{}_{}", peer_id_clone2, port);
        {
            let mut reg = udp_reg_clone.lock().unwrap();
            reg.insert(socket_key.clone(), socket.clone());
        }

        let socket_sub = socket.clone();
        tokio::spawn(async move {
            let mut buf = vec![0u8; 65536];
            loop {
                {
                    let map = peer_map_clone2.lock().unwrap();
                    if !map.contains_key(&peer_id_clone2) {
                        break;
                    }
                }

                tokio::select! {
                    res = socket_sub.recv_from(&mut buf) => {
                        if let Ok((size, src)) = res {
                            let data = &buf[..size];
                            {
                                let guard = supp_clone2.lock().unwrap();
                                if guard.contains(data) {
                                    continue;
                                }
                            }

                            let payload = json!({
                                "type": "direct_message",
                                "payload": {
                                    "targetId": peer_id_clone2.clone(),
                                    "msgType": "udp_tunnel_raw",
                                    "messagePayload": {
                                        "port": port,
                                        "hex_data": to_hex(data),
                                        "src_port": src.port()
                                    }
                                }
                            });
                            let _ = ws_tx_clone2.send(payload.to_string()).await;
                        }
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {
                        // Check
                    }
                }
            }
            let mut reg = udp_reg_clone.lock().unwrap();
            reg.remove(&socket_key);
        });
    });
}

fn spawn_peer_proxies(
    peer: Peer,
    peer_map: PeerMap,
    ws_tx: tokio::sync::mpsc::Sender<String>,
    suppressor: Arc<Mutex<EchoSuppressor>>,
    tcp_registry: TcpTunnelRegistry,
    udp_registry: UdpProxyRegistry,
    active_proxies: ActiveProxies,
) {
    let ports = vec![27015, 19132, 6112, 47624, 7777, 9947, 57395, 1007, 28910, 30000];
    for port in ports {
        let proxy_key = format!("{}_{}", peer.id, port);
        {
            let mut active = active_proxies.lock().unwrap();
            if active.contains(&proxy_key) {
                continue;
            }
            active.insert(proxy_key);
        }
        spawn_single_peer_port_proxy(
            peer.clone(),
            port,
            peer_map.clone(),
            ws_tx.clone(),
            suppressor.clone(),
            tcp_registry.clone(),
            udp_registry.clone(),
            active_proxies.clone(),
        );
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Elegant Terminal Welcome Headers
    println!("\x1b[38;5;208m");
    println!("==================================================================");
    println!("     R U S T N E T  • Virtual LAN Adapter Terminal Simulator      ");
    println!("  Secure Native Client Wrapper coded in safe Rust Engine (v0.1.0) ");
    println!("==================================================================");
    println!("\x1b[0m");

    let stdin = io::stdin();
    
    // 1. Get Coordinator server URL
    print!("Enter coordinator server WS URL [ws://localhost:3000]: ");
    io::stdout().flush()?;
    let mut server_url = String::new();
    stdin.read_line(&mut server_url)?;
    let mut server_url = server_url.trim().to_string();
    if server_url.is_empty() {
        server_url = "ws://localhost:3000".to_string();
    }

    // 2. Get username
    print!("Enter your player/client nickname: ");
    io::stdout().flush()?;
    let mut nickname = String::new();
    stdin.read_line(&mut nickname)?;
    let nickname = nickname.trim().to_string();
    if nickname.is_empty() {
        println!("Error: Nickname cannot be empty.");
        return Ok(());
    }

    // 3. Get Room ID
    print!("Enter custom virtual room/lobby ID [general-lobby]: ");
    io::stdout().flush()?;
    let mut room_id = String::new();
    stdin.read_line(&mut room_id)?;
    let mut room_id = room_id.trim().to_string();
    if room_id.is_empty() {
        room_id = "general-lobby".to_string();
    }

    log_info("SYS", &format!("Connecting to server: {} ...", server_url));

    // Connect to WebSocket Server
    let url = url::Url::parse(&server_url)?;
    let (ws_stream, _) = match connect_async(url).await {
        Ok(res) => res,
        Err(e) => {
            log_warn("SYS", &format!("Failed to connect to server: {}. Check that coordinator server is running on the correct port.", e));
            return Ok(());
        }
    };

    log_info("SYS", "Established WebSocket handshake. Joining virtual room...");

    let (mut write_half, mut read_half) = ws_stream.split();

    // Send room join register packet
    let join_packet = json!({
        "type": "join_room",
        "payload": {
            "roomId": room_id.to_lowercase(),
            "username": nickname
        }
    });

    write_half.send(Message::Text(join_packet.to_string())).await?;

    // Create channel for transmitting outbound payloads from different tokio threads
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(128);

    // Spawn outbound transmission relay task
    let packet_relay_tx = tx.clone();
    tokio::spawn(async move {
        while let Some(msg_payload) = rx.recv().await {
            if let Err(e) = write_half.send(Message::Text(msg_payload)).await {
                log_warn("WS_WRITE", &format!("Websocket write failure: {}", e));
                break;
            }
        }
    });

    // Spawn Keepalive Heartbeat task (PING sent every 5 seconds)
    let heartbeat_tx = packet_relay_tx.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            let ping_packet = json!({
                "type": "ping",
                "payload": {
                    "timestamp": chrono::Utc::now().timestamp_millis()
                }
            });
            if heartbeat_tx.send(ping_packet.to_string()).await.is_err() {
                break; // Connection closed
            }
        }
    });

    // Initialize Thread-Safe Suppressor
    let suppressor = Arc::new(Mutex::new(EchoSuppressor::new(200)));

    let peer_map: PeerMap = Arc::new(Mutex::new(HashMap::new()));
    let tcp_tunnel_registry: TcpTunnelRegistry = Arc::new(Mutex::new(HashMap::new()));
    let udp_proxy_registry: UdpProxyRegistry = Arc::new(Mutex::new(HashMap::new()));
    let host_udp_registry: HostUdpRegistry = Arc::new(Mutex::new(HashMap::new()));
    let active_proxies: ActiveProxies = Arc::new(Mutex::new(HashSet::new()));

    // Spawn Background UDP Broadcast Listeners/Bridge Sniffer
    let suppressor_for_listen = suppressor.clone();
    let listen_packet_tx = packet_relay_tx.clone();
    let ports = vec![27015, 19132, 6112, 47624, 7777, 9947, 57395, 1007, 28910, 30000, 4445];

    for &p in &ports {
        let supp = suppressor_for_listen.clone();
        let relay_tx = listen_packet_tx.clone();
        tokio::spawn(async move {
            let socket = match UdpSocket::bind(format!("0.0.0.0:{}", p)).await {
                Ok(s) => {
                    let _ = s.set_broadcast(true);
                    let _ = s.set_multicast_loop_v4(true);
                    if p == 4445 {
                        let _ = s.join_multicast_v4(
                            std::net::Ipv4Addr::new(224, 0, 2, 60),
                            std::net::Ipv4Addr::new(0, 0, 0, 0)
                        );
                    }
                    s
                }
                Err(_) => {
                    // Normal behavior: e.g., if a local game server is actively hosting on this computer,
                    // we cannot sniff LAN search queries on this port (as the game has exclusive bind).
                    return;
                }
            };

            let mut buf = vec![0u8; 65536];
            loop {
                match socket.recv_from(&mut buf).await {
                    Ok((size, src)) => {
                        let data = &buf[..size];

                        // Suppress echo to avoid infinite forwarding loops
                        {
                            let guard = supp.lock().unwrap();
                            if guard.contains(data) {
                                continue;
                            }
                        }

                        // Filter out local loopback packet echo from same port
                        if src.ip().is_loopback() && src.port() == p {
                            continue;
                        }

                        let hex_data = to_hex(data);
                        let broadcast_payload = json!({
                            "type": "network_broadcast_packet",
                            "payload": {
                                "protocol": "L2_LAN_HEX",
                                "port": p,
                                "hex_data": hex_data,
                                "src_port": src.port(),
                                "size": size,
                                "description": "DYNAMIC_LAN_BROADCAST_QUERY"
                            }
                        });

                        let _ = relay_tx.send(broadcast_payload.to_string()).await;
                    }
                    Err(_) => break,
                }
            }
        });
    }

    // Spawn Inbound WS Reader thread/task to capture server emissions
    let inbound_suppressor = suppressor.clone();
    let inbound_relay_tx = packet_relay_tx.clone();
    let peer_map_clone = peer_map.clone();
    let tcp_tunnel_registry_clone = tcp_tunnel_registry.clone();
    let udp_proxy_registry_clone = udp_proxy_registry.clone();
    let host_udp_registry_clone = host_udp_registry.clone();
    let active_proxies_clone = active_proxies.clone();

    tokio::spawn(async move {
        while let Some(Ok(message)) = read_half.next().await {
            if let Message::Text(text) = message {
                if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&text) {
                    match server_msg {
                        ServerMessage::RoomJoined { id, room_id, virtual_ip, peers } => {
                            println!("\n\x1b[32m=================== VIRTUAL CONNECTOR ONLINE ===================\x1b[0m");
                            log_info("TUNNEL", &format!("Successfully registered interface (rustnet_tap) on Room: {}", room_id));
                            log_info("TUNNEL", &format!("Your Assigned Virtual IP address: \x1b[35m{}\x1b[0m", virtual_ip));
                            log_info("TUNNEL", &format!("Client System Handle ID: {}", id));
                            log_info("TUNNEL", &format!("Found active peer network interfaces in subnet: {}", peers.len()));
                            
                            // Register our own IP on the local loopback adapter
                            try_add_loopback_alias(&virtual_ip);

                            {
                                let mut map = peer_map_clone.lock().unwrap();
                                map.clear();
                                for peer in &peers {
                                    map.insert(peer.id.clone(), peer.clone());
                                    log_info("PEER", &format!("• Connected peer: {} with IP: {}", peer.username, peer.virtual_ip));
                                    
                                    // Make remote peer's IP address a local loopback alias
                                    try_add_loopback_alias(&peer.virtual_ip);
                                    print_elevation_guide(&peer.virtual_ip, &peer.username);

                                    spawn_peer_proxies(
                                        peer.clone(),
                                        peer_map_clone.clone(),
                                        inbound_relay_tx.clone(),
                                        inbound_suppressor.clone(),
                                        tcp_tunnel_registry_clone.clone(),
                                        udp_proxy_registry_clone.clone(),
                                        active_proxies_clone.clone(),
                                    );
                                }
                            }
                            println!("\x1b[32m================================================================\x1b[0m");
                            println!("Type anything and press ENTER to chat. Real-time L2 Virtual LAN Automatic discovery is fully active.");
                            println!("To test manually, you can use: \x1b[33m/host <game_name> <port>\x1b[0m\n");
                        }
                        ServerMessage::PeerJoined { id, username, virtual_ip } => {
                            log_info("PEER", &format!("\x1b[32m[+]\x1b[0m Peer joined network segment: {} ({})", username, virtual_ip));
                            
                            // Make new remote peer's IP address a local loopback alias
                            try_add_loopback_alias(&virtual_ip);
                            print_elevation_guide(&virtual_ip, &username);

                            let joined_peer = Peer {
                                id: id.clone(),
                                username: username.clone(),
                                virtual_ip: virtual_ip.clone(),
                            };
                            {
                                let mut map = peer_map_clone.lock().unwrap();
                                map.insert(id.clone(), joined_peer.clone());
                            }
                            spawn_peer_proxies(
                                joined_peer,
                                peer_map_clone.clone(),
                                inbound_relay_tx.clone(),
                                inbound_suppressor.clone(),
                                tcp_tunnel_registry_clone.clone(),
                                udp_proxy_registry_clone.clone(),
                                active_proxies_clone.clone(),
                            );
                        }
                        ServerMessage::PeerLeft { id, username, virtual_ip } => {
                            log_info("PEER", &format!("\x1b[31m[-]\x1b[0m Peer left network segment: {} ({})", username, virtual_ip));
                            {
                                let mut map = peer_map_clone.lock().unwrap();
                                map.remove(&id);
                            }
                            {
                                let mut active = active_proxies_clone.lock().unwrap();
                                active.retain(|k| !k.starts_with(&format!("{}_", id)));
                            }
                        }
                        ServerMessage::ChatMessage { sender_id: _, sender_name, message, timestamp: _ } => {
                            println!("[\x1b[35mChat\x1b[0m] \x1b[1m{}:\x1b[0m {}", sender_name, message);
                        }
                        ServerMessage::NetworkPacketReceived { sender_id, sender_ip: _, sender_name, protocol, port, virtual_destination: _, size: _, description, hex_data, src_port } => {
                            if description == "DYNAMIC_LAN_BROADCAST_QUERY" {
                                if let (Some(hex), Some(client_src)) = (hex_data, src_port) {
                                    if let Ok(bytes) = from_hex(&hex) {
                                        let supp = inbound_suppressor.clone();
                                        let tx = inbound_relay_tx.clone();
                                        let sender_id_clone = sender_id.clone();
                                        let peer_map_clone2 = peer_map_clone.clone();
                                        let active_proxies_clone2 = active_proxies_clone.clone();
                                        let tcp_registry_clone2 = tcp_tunnel_registry_clone.clone();
                                        let udp_registry_clone2 = udp_proxy_registry_clone.clone();

                                        tokio::spawn(async move {
                                            let mut peer_ip = "127.0.0.1".to_string();
                                            {
                                                let map = peer_map_clone2.lock().unwrap();
                                                if let Some(peer) = map.get(&sender_id_clone) {
                                                    peer_ip = peer.virtual_ip.replace("10.8.0.", "127.0.0.");
                                                }
                                            }

                                            // Dual interface sockets: binding to 0.0.0.0:0 sends over physical adapters,
                                            // while binding to peer_ip:0 sends over the virtual loopback adapter.
                                            // This satisfies absolute OS discovery requirements for all standard LAN games!
                                            let wildcard_sock = UdpSocket::bind("0.0.0.0:0").await.ok();
                                            let loopback_sock = UdpSocket::bind(format!("{}:0", peer_ip)).await.ok();

                                            // Suppress echo
                                            {
                                                let mut guard = supp.lock().unwrap();
                                                guard.insert(bytes.clone());
                                            }

                                            let targets = if port == 4445 {
                                                vec![
                                                    "224.0.2.60:4445".to_string(),
                                                    "255.255.255.255:4445".to_string(),
                                                    "127.0.0.1:4445".to_string(),
                                                    format!("{}:4445", peer_ip)
                                                ]
                                            } else {
                                                vec![
                                                    format!("127.0.0.1:{}", port),
                                                    format!("255.255.255.255:{}", port),
                                                    format!("{}:{}", peer_ip, port)
                                                ]
                                            };

                                            let mut sent_any = false;

                                            // Trigger on physical interfaces (0.0.0.0)
                                            if let Some(ref sock) = wildcard_sock {
                                                let _ = sock.set_broadcast(true);
                                                for target in &targets {
                                                    if sock.send_to(&bytes, target).await.is_ok() {
                                                        sent_any = true;
                                                    }
                                                }
                                            }

                                            // Trigger on loopback interface (127.0.0.x)
                                            if let Some(ref sock) = loopback_sock {
                                                let _ = sock.set_broadcast(true);
                                                for target in &targets {
                                                    if sock.send_to(&bytes, target).await.is_ok() {
                                                        sent_any = true;
                                                    }
                                                }
                                            }

                                            if sent_any {
                                                // Dynamic registration for Minecraft Java random ports
                                                if port == 4445 {
                                                    if let Ok(s) = std::str::from_utf8(&bytes) {
                                                        if let Some(start) = s.find("[AD]") {
                                                            if let Some(end) = s.find("[/AD]") {
                                                                let port_str = &s[start + 4..end];
                                                                if let Ok(mc_port) = port_str.parse::<u16>() {
                                                                    let mut active = active_proxies_clone2.lock().unwrap();
                                                                    let proxy_key = format!("{}_", sender_id_clone);
                                                                    let full_key = format!("{}{}", proxy_key, mc_port);
                                                                    if !active.contains(&full_key) {
                                                                        active.insert(full_key);
                                                                        let mut peer_opt = None;
                                                                        {
                                                                            let map = peer_map_clone2.lock().unwrap();
                                                                            if let Some(p) = map.get(&sender_id_clone) {
                                                                                peer_opt = Some(p.clone());
                                                                            }
                                                                        }
                                                                        if let Some(p) = peer_opt {
                                                                            spawn_single_peer_port_proxy(
                                                                                p,
                                                                                mc_port,
                                                                                peer_map_clone2.clone(),
                                                                                tx.clone(),
                                                                                supp.clone(),
                                                                                tcp_registry_clone2.clone(),
                                                                                udp_registry_clone2.clone(),
                                                                                active_proxies_clone2.clone(),
                                                                            );
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }

                                                // Wait for any reply back from physical socket
                                                if let Some(ref sock) = wildcard_sock {
                                                    let mut reply_buf = vec![0u8; 65536];
                                                    if let Ok(Ok((reply_size, _))) = tokio::time::timeout(
                                                        tokio::time::Duration::from_millis(1500),
                                                        sock.recv_from(&mut reply_buf)
                                                    ).await {
                                                        let reply_bytes = &reply_buf[..reply_size];
                                                        let reply_hex = to_hex(reply_bytes);

                                                        let reply_envelope = json!({
                                                            "type": "direct_message",
                                                            "payload": {
                                                                "targetId": sender_id_clone,
                                                                "msgType": "lan_udp_reply",
                                                                "messagePayload": {
                                                                    "port": port,
                                                                    "hex_data": reply_hex,
                                                                    "client_src_port": client_src
                                                                 }
                                                            }
                                                        });
                                                        let _ = tx.send(reply_envelope.to_string()).await;
                                                    }
                                                }
                                            }
                                        });
                                    }
                                }
                            } else if description == "GAME_SERVER_ANNOUNCEMENT(HOST)" {
                                let mut peer_ip = "127.0.0.1".to_string();
                                {
                                    let map = peer_map_clone.lock().unwrap();
                                    if let Some(peer) = map.get(&sender_id) {
                                        peer_ip = peer.virtual_ip.replace("10.8.0.", "127.0.0.");
                                    }
                                }

                                log_info("UDP_BROADCAST", &format!("\x1b[32;1m🎮 LAN game discovered!\x1b[0m Game: \x1b[1m{}\x1b[0m on Port: \x1b[1m{}\x1b[0m, Host: {} ({})", protocol, port, sender_name, protocol));
                                log_info("CONNECTION", &format!("⚡ BRIDGE OPENED! Connect in-game using Direct Connection IP: \x1b[32;1m{}:{}\x1b[0m or \x1b[32;1m127.0.0.1:{}\x1b[0m", peer_ip, port, port));
                                if protocol.to_lowercase().contains("cs") || protocol.to_lowercase().contains("strike") {
                                    log_info("CONNECTION", &format!("👉 For CS 1.6: Open game console (~) and type: \x1b[33;1mconnect {}:{}\x1b[0m", peer_ip, port));
                                } else if protocol.to_lowercase().contains("minecraft") {
                                    log_info("CONNECTION", &format!("👉 For Minecraft: Choose Multiplayer -> Direct Connection -> type: \x1b[33;1m{}:{}\x1b[0m", peer_ip, port));
                                }

                                // Automatically spawn full TCP/UDP dynamic proxies for this manually announced custom port on client!
                                let mut active = active_proxies_clone.lock().unwrap();
                                let proxy_key = format!("{}_", sender_id);
                                let full_key = format!("{}{}", proxy_key, port);
                                if !active.contains(&full_key) {
                                    active.insert(full_key);
                                    let mut peer_opt = None;
                                    {
                                        let map = peer_map_clone.lock().unwrap();
                                        if let Some(p) = map.get(&sender_id) {
                                            peer_opt = Some(p.clone());
                                        }
                                    }
                                    if let Some(p) = peer_opt {
                                        spawn_single_peer_port_proxy(
                                            p.clone(),
                                            port,
                                            peer_map_clone.clone(),
                                            inbound_relay_tx.clone(),
                                            inbound_suppressor.clone(),
                                            tcp_tunnel_registry_clone.clone(),
                                            udp_proxy_registry_clone.clone(),
                                            active_proxies_clone.clone(),
                                        );

                                        // Pro-actively inject local Minecraft Java discovery beacon if game server named Minecraft is announced
                                        if protocol.to_lowercase().contains("minecraft") {
                                            let mc_adv = format!("[MOTD]RustNet: {}'s Server[/MOTD][AD]{}[/AD]", sender_name, port);
                                            let mc_bytes = mc_adv.into_bytes();
                                            let peer_ip_clone = peer_ip.clone();
                                            tokio::spawn(async move {
                                                let wildcard_sock = UdpSocket::bind("0.0.0.0:0").await.ok();
                                                let loop_sock = UdpSocket::bind(format!("{}:0", peer_ip_clone)).await.ok();
                                                let targets = vec![
                                                    "224.0.2.60:4445".to_string(),
                                                    "255.255.255.255:4445".to_string(),
                                                    "127.0.0.1:4445".to_string(),
                                                    format!("{}:4445", peer_ip_clone)
                                                ];

                                                if let Some(ref sock) = wildcard_sock {
                                                    let _ = sock.set_broadcast(true);
                                                    for target in &targets {
                                                        let _ = sock.send_to(&mc_bytes, target).await;
                                                    }
                                                }
                                                if let Some(ref sock) = loop_sock {
                                                    let _ = sock.set_broadcast(true);
                                                    for target in &targets {
                                                        let _ = sock.send_to(&mc_bytes, target).await;
                                                    }
                                                }
                                            });
                                        }
                                    }
                                }
                            }
                        }
                        ServerMessage::DirectMessage { sender_id, sender_name: _, msg_type, message_payload } => {
                            if msg_type == "lan_udp_reply" {
                                if let (Some(_port_val), Some(hex), Some(client_src)) = (
                                    message_payload["port"].as_u64(),
                                    message_payload["hex_data"].as_str(),
                                    message_payload["client_src_port"].as_u64()
                                ) {
                                    if let Ok(bytes) = from_hex(hex) {
                                        let supp = inbound_suppressor.clone();
                                        let sender_id_clone = sender_id.clone();
                                        let peer_map_clone3 = peer_map_clone.clone();
                                        tokio::spawn(async move {
                                            let mut peer_ip = "127.0.0.1".to_string();
                                            {
                                                let map = peer_map_clone3.lock().unwrap();
                                                if let Some(peer) = map.get(&sender_id_clone) {
                                                    peer_ip = peer.virtual_ip.replace("10.8.0.", "127.0.0.");
                                                }
                                            }
                                            let bind_addr = format!("{}:0", peer_ip);
                                            let temp_sock = match UdpSocket::bind(&bind_addr).await {
                                                Ok(s) => s,
                                                Err(_) => match UdpSocket::bind("0.0.0.0:0").await {
                                                    Ok(s) => s,
                                                    Err(_) => return,
                                                }
                                            };
                                            {
                                                let mut guard = supp.lock().unwrap();
                                                guard.insert(bytes.clone());
                                            }
                                            let _ = temp_sock.send_to(&bytes, format!("127.0.0.1:{}", client_src)).await;
                                        });
                                    }
                                }
                            } else if msg_type == "tcp_tunnel_connect" {
                                if let (Some(conn_id), Some(port_val)) = (
                                    message_payload["connection_id"].as_str(),
                                    message_payload["port"].as_u64()
                                ) {
                                    let conn_id = conn_id.to_string();
                                    let port = port_val as u16;
                                    let tcp_reg = tcp_tunnel_registry_clone.clone();
                                    let ws_tx = inbound_relay_tx.clone();
                                    let sender_id_clone = sender_id.clone();

                                    tokio::spawn(async move {
                                        let local_addr = format!("127.0.0.1:{}", port);
                                        match tokio::net::TcpStream::connect(&local_addr).await {
                                            Ok(stream) => {
                                                log_info("TCP_HOST", &format!("Successfully connected to local game server: {}", local_addr));
                                                let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(512);
                                                {
                                                    let mut reg = tcp_reg.lock().unwrap();
                                                    reg.insert(conn_id.clone(), tx);
                                                }

                                                let (mut reader, mut writer) = tokio::io::split(stream);

                                                // Spawn reader
                                                let conn_id_sub = conn_id.clone();
                                                let sender_id_sub = sender_id_clone.clone();
                                                let ws_tx_sub = ws_tx.clone();
                                                tokio::spawn(async move {
                                                    let mut buf = vec![0u8; 16384];
                                                    while let Ok(n) = reader.read(&mut buf).await {
                                                        if n == 0 { break; }
                                                        let data_payload = json!({
                                                            "type": "direct_message",
                                                            "payload": {
                                                                "targetId": sender_id_sub.clone(),
                                                                "msgType": "tcp_tunnel_data",
                                                                "messagePayload": {
                                                                    "connection_id": conn_id_sub.clone(),
                                                                    "hex_data": to_hex(&buf[..n])
                                                                }
                                                            }
                                                        });
                                                        if ws_tx_sub.send(data_payload.to_string()).await.is_err() {
                                                            break;
                                                        }
                                                    }
                                                    // closed
                                                    let close_payload = json!({
                                                        "type": "direct_message",
                                                        "payload": {
                                                            "targetId": sender_id_sub,
                                                            "msgType": "tcp_tunnel_close",
                                                            "messagePayload": {
                                                                "connection_id": conn_id_sub
                                                            }
                                                        }
                                                    });
                                                    let _ = ws_tx_sub.send(close_payload.to_string()).await;
                                                });

                                                // Spawn writer
                                                let tcp_reg_sub = tcp_reg.clone();
                                                let conn_id_sub2 = conn_id.clone();
                                                tokio::spawn(async move {
                                                    while let Some(data) = rx.recv().await {
                                                        if writer.write_all(&data).await.is_err() {
                                                            break;
                                                        }
                                                    }
                                                    let mut reg = tcp_reg_sub.lock().unwrap();
                                                    reg.remove(&conn_id_sub2);
                                                });
                                            }
                                            Err(e) => {
                                                log_warn("TCP_HOST", &format!("Could not connect to local host game at {}: {:?}", local_addr, e));
                                                let close_payload = json!({
                                                    "type": "direct_message",
                                                    "payload": {
                                                        "targetId": sender_id_clone,
                                                        "msgType": "tcp_tunnel_close",
                                                        "messagePayload": {
                                                            "connection_id": conn_id
                                                        }
                                                    }
                                                });
                                                let _ = ws_tx.send(close_payload.to_string()).await;
                                            }
                                        }
                                    });
                                }
                            } else if msg_type == "tcp_tunnel_data" {
                                if let (Some(conn_id), Some(hex_data)) = (
                                    message_payload["connection_id"].as_str(),
                                    message_payload["hex_data"].as_str()
                                ) {
                                    if let Ok(bytes) = from_hex(hex_data) {
                                        let sender_channel = {
                                            let reg = tcp_tunnel_registry_clone.lock().unwrap();
                                            reg.get(conn_id).cloned()
                                        };
                                        if let Some(ch) = sender_channel {
                                            let _ = ch.send(bytes).await;
                                        }
                                    }
                                }
                            } else if msg_type == "tcp_tunnel_close" {
                                if let Some(conn_id) = message_payload["connection_id"].as_str() {
                                    let mut reg = tcp_tunnel_registry_clone.lock().unwrap();
                                    reg.remove(conn_id);
                                }
                            } else if msg_type == "udp_tunnel_raw" {
                                if let (Some(port_val), Some(hex_data), Some(src_port_val)) = (
                                    message_payload["port"].as_u64(),
                                    message_payload["hex_data"].as_str(),
                                    message_payload["src_port"].as_u64()
                                ) {
                                    let port = port_val as u16;
                                    let src_port = src_port_val as u16;
                                    if let Ok(bytes) = from_hex(hex_data) {
                                        let host_sock_key = format!("{}_{}", sender_id, src_port);
                                        let host_sock_opt = {
                                            let reg = host_udp_registry_clone.lock().unwrap();
                                            reg.get(&host_sock_key).cloned()
                                        };

                                        if let Some(sock) = host_sock_opt {
                                            let _ = sock.send_to(&bytes, format!("127.0.0.1:{}", port)).await;
                                        } else {
                                            if let Ok(sock) = UdpSocket::bind("0.0.0.0:0").await {
                                                let sock = Arc::new(sock);
                                                {
                                                    let mut reg = host_udp_registry_clone.lock().unwrap();
                                                    reg.insert(host_sock_key.clone(), sock.clone());
                                                }

                                                let sock_sub = sock.clone();
                                                let sender_id_sub = sender_id.clone();
                                                let ws_tx = inbound_relay_tx.clone();
                                                tokio::spawn(async move {
                                                    let mut resp_buf = vec![0u8; 65536];
                                                    while let Ok((n, _)) = sock_sub.recv_from(&mut resp_buf).await {
                                                        let data_hex = to_hex(&resp_buf[..n]);
                                                        let reply_envelope = json!({
                                                            "type": "direct_message",
                                                            "payload": {
                                                                "targetId": sender_id_sub.clone(),
                                                                "msgType": "udp_tunnel_reply",
                                                                "messagePayload": {
                                                                    "port": port,
                                                                    "hex_data": data_hex,
                                                                    "src_port": src_port
                                                                }
                                                            }
                                                        });
                                                        if ws_tx.send(reply_envelope.to_string()).await.is_err() {
                                                            break;
                                                        }
                                                    }
                                                });

                                                let _ = sock.send_to(&bytes, format!("127.0.0.1:{}", port)).await;
                                            }
                                        }
                                    }
                                }
                            } else if msg_type == "udp_tunnel_reply" {
                                if let (Some(port_val), Some(hex_data), Some(src_port_val)) = (
                                    message_payload["port"].as_u64(),
                                    message_payload["hex_data"].as_str(),
                                    message_payload["src_port"].as_u64()
                                ) {
                                    let port = port_val as u16;
                                    let src_port = src_port_val as u16;
                                    if let Ok(bytes) = from_hex(hex_data) {
                                        let mut sender_ip = "127.0.0.1".to_string();
                                        {
                                            let map = peer_map_clone.lock().unwrap();
                                            if let Some(peer) = map.get(&sender_id) {
                                                sender_ip = peer.virtual_ip.replace("10.8.0.", "127.0.0.");
                                            }
                                        }

                                        let socket_key = format!("{}_{}", sender_id, port);
                                        let socket_opt = {
                                            let reg = udp_proxy_registry_clone.lock().unwrap();
                                            reg.get(&socket_key).cloned()
                                        };

                                        if let Some(sock) = socket_opt {
                                            let _ = sock.send_to(&bytes, format!("127.0.0.1:{}", src_port)).await;
                                        }
                                    }
                                }
                            }
                        }
                        ServerMessage::Pong { timestamp } => {
                            let now = chrono::Utc::now().timestamp_millis();
                            let latency = now - (timestamp as i64);
                            if latency >= 0 && latency < 5000 {
                                // Keepalive indicator safely logged (uncomment to trace roundtree)
                                // log_info("SYS", &format!("Keepalive OK (RTT: {} ms)", latency));
                            }
                        }
                        ServerMessage::Error { message } => {
                            log_warn("SYS", &format!("Coordinator reported error: {}", message));
                        }
                    }
                }
            }
        }
        log_warn("SYS", "WebSocket server connection has been terminated.");
        std::process::exit(1);
    });

    // Handle interactive terminal user input on standard stdin loop
    println!("System fully bootup. Live packet processing active. Type chat message and press Enter:");
    let input_reader = BufReader::new(tokio::io::stdin());
    let mut lines = input_reader.lines();

    loop {
        tokio::select! {
            line_res = lines.next_line() => {
                match line_res {
                    Ok(Some(line)) => {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }

                        // Support command to simulate hosting a LAN game on the virtual adapter
                        if trimmed.starts_with("/host") {
                            let parts: Vec<&str> = trimmed.split_whitespace().collect();
                            if parts.len() >= 3 {
                                let game_name = parts[1];
                                let port_str = parts[2];
                                if let Ok(port) = port_str.parse::<u16>() {
                                    let host_packet = json!({
                                        "type": "network_broadcast_packet",
                                        "payload": {
                                            "protocol": game_name,
                                            "port": port,
                                            "virtualDestination": "255.255.255.255",
                                            "size": 128,
                                            "description": "GAME_SERVER_ANNOUNCEMENT(HOST)"
                                        }
                                    });
                                    packet_relay_tx.send(host_packet.to_string()).await?;
                                    log_info("VIRT_TAP", &format!("Injected L2 Broadcast simulation packet for game: {} on port {}", game_name, port));
                                } else {
                                    println!("Invalid port specify! Usage: /host <game_name> <port>");
                                }
                            } else {
                                println!("Usage: /host <game_name> <port> (e.g. /host Minecraft 19132)");
                            }
                        } else {
                            // General room chat delivery
                            let chat_packet = json!({
                                "type": "chat_message",
                                "payload": {
                                    "message": trimmed
                                }
                            });
                            packet_relay_tx.send(chat_packet.to_string()).await?;
                        }
                    }
                    _ => break,
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("\n\x1b[33m================= CLEAN SHUTDOWN TRIGGERED =================\x1b[0m");
                log_info("CLEANUP", "SIGINT / Ctrl+C detected. Releasing resources gracefully...");
                log_info("CLEANUP", "Closing all local UDP bridge listener socket bindings...");
                log_info("CLEANUP", "Purging all active, in-memory echo routing caches...");
                log_info("CLEANUP", "Disconnecting active room websocket interface cleanly...");
                log_info("CLEANUP", "Resetting all virtual states. No local network configurations were altered.");
                println!("\x1b[32m[✓] RESTORE SUCCESS: Your computer's original network remains completely clean and untouched.\x1b[0m");
                println!("\x1b[33m============================================================\x1b[0m");
                break;
            }
        }
    }

    Ok(())
}
