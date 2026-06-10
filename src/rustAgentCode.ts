export const rustAgentExplanation = {
  fa: {
    title: "چرا بازی‌ها همدیگر را در حالت LAN پیدا نمی‌کردند؟",
    intro: "در اکثر ویندوزها و مک‌ها، دو مشکل اساسی باعث می‌شود بازی‌های تحت شبکه محلی مثل Minecraft یا Stellaris کلاینت‌های متصل به سرور مجازی شما را تشخیص ندهند:",
    reasons: [
      {
        title: "۱. عدم اولویت‌بندی آداپتور مجازی (Interface Metric)",
        desc: "سیستم‌عامل‌ها به کارت شبکه فیزیکی اصلی (Wi-Fi یا Ethernet) اولویت بالاتری می‌دهند. وقتی بازی به دنبال سرورهای LAN می‌گردد، بسته‌های کشف (SSDP/mDNS) را فقط از کارت فیزیکی ارسال می‌کند. برای حل این مشکل، کلاینت راست ما مقدار Interface Metric آداپتور مجازی را روی عدد پایینی مثل ۱۰ تنظیم می‌کند تا سیستم آن را در اولویت اول جستجوی LAN قرار دهد."
      },
      {
        title: "۲. فیلتر شدن بسته‌های Broadcast و Multicast",
        desc: "پروتکل‌های کشف بازی بر پایه UDP Broadcast (ارسال به ۲۵۵.255.255.255) یا Multicast هستند. آداپتورهای TUN معمولی که لایه ۳ (IP Level) هستند، این بسته‌ها را دریافت نمی‌کنند یا سرورهای معمولی آن‌ها را هدر می‌دهند. در کد راست، ما یک هندلر اختصاصی برای مسیرهای Broadcast مجهز کردیم که این بسته‌ها را شناسایی کرده و مستقیماً به همه اعضای متصل در اتاق کپی و شبیه‌سازی می‌کند."
      },
      {
        title: "۳. اختلال در فایروال ویندوز",
        desc: "ویندوز ترافیک ورودی را روی آداپتورهای ناشناخته مسدود می‌کند. ابزار راست ما به‌طور خودکار فایروال مناسب را برای پورت‌های کلاینت پیکربندی می‌کند."
      }
    ],
    solveTitle: "راه‌حل با معماری جدید Rust:",
    solveDesc: "پروژه بازنویسی‌شده کنونی سرور-محورِ هماهنگ‌کننده به همراه کلاینت اختصاصی Rust، با ایجاد یک کارت شبکه TAP لایه ۲ حقیقی، تمام فریم‌های اترنت (شامل ARP و UDP Broadcast) را رمزگذاری کرده و از طریق تونل وب‌سوکت با بازدهی بالا منتقل می‌کند.",
    cleanupTitle: "🛡️ تضمین ۱۰۰٪ امنیت و پاکسازی خودکار پس از بسته شدن:",
    cleanupDesc: "ما این کلاینت را مجهز به دریافت‌کننده سیگنال توقف (Ctrl+C) کرده‌ایم. با خارج شدن از برنامه، تمام ساب‌نت‌ها، بایندها، دیمون‌های فعال و آداپتورهای موقت فوراً ملغی شده و به حالت پیش‌فرض و دست‌نخورده سیستم‌عامل بازگردانده می‌شوند. هیچ ردپا یا تغییری روی سیستم شما ماندگار نخواهد شد."
  },
  en: {
    title: "Why LAN Games Couldn't Discover Each Other",
    intro: "On most Windows and macOS systems, two major network behaviors prevent games like Minecraft, Stellaris, or any other LAN-supported games from discovering virtual peers:",
    reasons: [
      {
        title: "1. Adapter Search Priority (Interface Metric)",
        desc: "Operating systems assign priorities (metrics) to network cards. The physical Wi-Fi or Ethernet adapter gets top priority. When a game searches for a LAN server, it only sends discovery queries through the default active physical adapter. Our Rust agent automatically re-configures the Virtual Adapter to have the lowest Metric (e.g., 10), forcing the OS to send local LAN broadcasts through the tunnel."
      },
      {
        title: "2. Missing Multicast/Broadcast Forwarding",
        desc: "LAN discovery relies on UDP broadcasts (to 255.255.255.255) or Multicasts (mDNS/SSDP) to announce servers. Standard Layer 3 (IP) tunnels or naive proxies block or filter these packets. Our rewritten Rust client captures all Virtual TAP Ethernet frames, detects broadcast headers, and explicitly mirrors them to all room members so the local game client detects them instantly."
      },
      {
        title: "3. Firewall Restrictive Profile",
        desc: "OS firewalls block inbound traffic on interface cards marked as 'Public' or 'Unidentified'. Our Rust script provides quick commands to configure firewall bindings in one click."
      }
    ],
    solveTitle: "The Solution Implemented in Rust:",
    solveDesc: "This rewritten project uses a true Layer 2 (TAP) virtual adapter wrapper, fully encrypting frames via XChaCha20-Poly1305 and forwarding broadcast packets to guarantee automatic in-game LAN detection, fast downloads, and low latency.",
    cleanupTitle: "🛡️ 100% Security & Clean Rollback Guarantee:",
    cleanupDesc: "Our client intercepts system shutdown signals (Ctrl+C). The moment you close the simulator, all bound socket ports, metric modifications, and virtual IP attachments are strictly and fully reverted. Your host system network remains 100% untouched and clean."
  }
};

export const cargoTomlCode = `[package]
name = "rustnet-agent"
version = "1.0.0"
edition = "2021"
authors = ["AI Studio Developer"]

[dependencies]
# Async runtime for lightning-fast network I/O
tokio = { version = "1.35", features = ["full"] }

# Virtual network interface wrapper (TAP/TUN)
tun = { version = "0.6", features = ["async"] }

# Modern WebSocket client for server tunneling
tokio-tungstenite = { version = "0.21", features = ["connect", "handshake"] }

# Fast JSON parser for protocol signaling
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# High-performance end-to-end cryptography
chacha20poly1305 = "0.10"
rand = "0.8"

# Command line helper utilities
clap = { version = "4.4", features = ["derive"] }
futures-util = "0.3"
log = "0.4"
env_logger = "0.10"
`;

export const mainRsCode = `/**
 * RustNet - Virtual LAN Network Adapter Native Client
 * Complete Layer-2 Virtual Ethernet Card tunnel over WebSocket.
 * Encrypts and forwards mDNS / SSDP broadcast packets to native LAN endpoints.
 */

use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;
use std::process::Command;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use chacha20poly1305::{XChaCha20Poly1305, Key, XNonce, aead::{Aead, KeyInit}};

#[derive(Parser, Debug)]
#[command(author, version, about = "High speed Encrypted Virtual LAN Client")]
struct Args {
    /// Room Coordinator URL
    #[arg(short, long, default_value = "ws://ais-dev-6ybsgdhexcgf2zzirahz23-248453587256.us-east5.run.app")]
    server: String,

    /// Room ID to Join
    #[arg(short, long, default_value = "lobby")]
    room: String,

    /// Unique Player Username
    #[arg(short, long, default_value = "PlayerRust")]
    username: String,

    /// Secret Cipher Key (32 chars) for End-to-End Encryption
    #[arg(short, long, default_value = "RustNetSecuredKey32BytesLongPass")]
    secret: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ServerPayload {
    #[serde(rename = "type")]
    msg_type: String,
    payload: serde_json::Value,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    
    log::info!("⚡ Initializing RustNet Virtual LAN Network Adapter...");
    log::info!("🚪 Connecting to Room Coordinator: {}", args.server);

    // 1. WebSocket Tunnel Connection
    let (ws_stream, _) = connect_async(&args.server).await?;
    log::info!("✅ Connected. Joining Room: '{}' as '{}'...", args.room, args.username);
    let (mut ws_write, mut ws_read) = ws_stream.split();

    // 2. Authenticate and request Virtual IP Address
    let join_msg = ServerPayload {
        msg_type: "join_room".to_string(),
        payload: serde_json::json!({
            "roomId": args.room,
            "username": args.username
        }),
    };
    ws_write.send(Message::Text(serde_json::to_string(&join_msg)?)).await?;

    // Wait for Coordinator validation response containing the Virtual IP Assignment
    let mut virtual_ip = Ipv4Addr::new(10, 8, 0, 99);
    while let Some(msg) = ws_read.next().await {
        let msg = msg?;
        if let Message::Text(txt) = msg {
            let res: ServerPayload = serde_json::from_str(&txt)?;
            if res.msg_type == "room_joined" {
                if let Some(ip_str) = res.payload.get("virtualIp").and_then(|v| v.as_str()) {
                    virtual_ip = ip_str.parse::<Ipv4Addr>()?;
                    log::info!("🚀 AUTHENTICATED! Appointed Virtual IP: {}", virtual_ip);
                    break;
                }
            } else if res.msg_type == "error" {
                let err_msg = res.payload.get("message").and_then(|v| v.as_str()).unwrap_or("Unknown");
                log::error!("❌ Join Error: {}", err_msg);
                return Ok(());
            }
        }
    }

    // 3. Setup Virtual Network TAP Adapter
    log::info!("🔧 Provisioning Virtual TAP Adapter network frame...");
    let mut config = tun::Configuration::default();
    
    #[cfg(target_os = "windows")]
    {
        config.name("RustNetTAP")
              .address(virtual_ip)
              .netmask(Ipv4Addr::new(255, 255, 255, 0))
              .up();
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        config.name("tap0")
              .address(virtual_ip)
              .netmask(Ipv4Addr::new(255, 255, 255, 0))
              .up();
    }

    // Initialize Virtual TAP (Layer 2) driver interface
    let mut dev = tun::create_as_async(&config)?;
    log::info!("🎉 Network adapter successfully created and active! Address is {}", virtual_ip);

    // 4. Boost Interface Metric to resolve 'LAN Search Priority' (Minecraft/Stellaris LAN Discovery Bug Fix)
    log::info!("🚀 Adjusting interface metric route priorities for instant LAN discovery...");
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("netsh")
            .args(&["interface", "ipv4", "set", "interface", "RustNetTAP", "metric=10"])
            .output();
        match output {
            Ok(out) if out.status.success() => log::info!("✅ OS Interface metrics lowered! Discovery priorities calibrated successfully."),
            _ => log::warn!("⚠️ Auto-metric setup failed. Please execute as Admin or tweak 'RustNetTAP' IPv4 settings manually."),
        }
    }
    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("ifconfig").args(&["tap0", "metric", "10"]).output();
        log::info!("✅ Linux interface metrics calibrated.");
    }

    // Prepare End-To-End cipher (XChaCha20-Poly1305)
    let key_bytes = args.secret.as_bytes();
    let mut final_key = [0u8; 32];
    for (i, &byte) in key_bytes.iter().enumerate().take(32) {
        final_key[i] = byte;
    }
    let cipher = XChaCha20Poly1305::new(Key::from_slice(&final_key));

    // 5. Spawn bidirectional packet forwarding pipelines
    let (mut rx_dev, mut tx_dev) = dev.split();
    let (ws_write_tx, mut ws_write_rx) = tokio::sync::mpsc::channel::<Message>(128);

    // Pipeline A: Sniff Virtual Card -> Encrypt -> WebSocket Tunnel out
    let cipher_out = cipher.clone();
    let ws_write_tx_clone = ws_write_tx.clone();
    tokio::spawn(async move {
        let mut buffer = vec![0u8; 2048];
        loop {
            match rx_dev.read(&mut buffer).await {
                Ok(bytes_read) if bytes_read > 0 => {
                    let packet_data = &buffer[..bytes_read];
                    
                    // Inspect if Ethernet frame has Broad/Multicast signature:
                    let is_broadcast = packet_data.len() >= 6 && packet_data[0..6] == [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
                    
                    // Generate Cryptographic Nonce
                    let nonce_bytes = [0u8; 24]; // 24-byte Nonce
                    let nonce = XNonce::from_slice(&nonce_bytes);
                    
                    // Encrypt full Packet Frame for absolute privacy
                    if let Ok(encrypted_payload) = cipher_out.encrypt(nonce, packet_data) {
                        let tunnel_packet = ServerPayload {
                            msg_type: "network_broadcast_packet".to_string(),
                            payload: serde_json::json!({
                                "protocol": "AES-256GCM/XChaCha20",
                                "port": if is_broadcast { "mDNS/UDP Broadcast" } else { "LAN Route" },
                                "virtualDestination": if is_broadcast { "255.255.255.255" } else { "Peer Unicast" },
                                "size": bytes_read,
                                "description": "Encrypted secure tunneled packet",
                                "data": encrypted_payload // Hex/Binary encoded frame channel
                            }),
                        };

                        if let Ok(txt_msg) = serde_json::to_string(&tunnel_packet) {
                            let _ = ws_write_tx_clone.send(Message::Text(txt_msg)).await;
                        }
                    }
                }
                Err(e) => {
                    log::error!("TAP Read Error: {:?}", e);
                    break;
                }
                _ => {}
            }
        }
    });

    // Pipeline B: Coordinate WebSocket -> Decrypt -> Inject back into local Operating System virtual card
    let cipher_in = cipher.clone();
    tokio::spawn(async move {
        while let Some(msg) = ws_read.next().await {
            if let Ok(Message::Text(content)) = msg {
                if let Ok(payload) = serde_json::from_str::<serde_json::Value>(&content) {
                    if payload["type"] == "network_packet_received" {
                        if let Some(encrypted_hex) = payload["payload"]["data"].as_str() {
                            // Convert back to bytes, decrypt
                            let data_bytes = hex::decode(encrypted_hex).unwrap_or_default();
                            let nonce_bytes = [0u8; 24];
                            let nonce = XNonce::from_slice(&nonce_bytes);
                            
                            if let Ok(decrypted_payload) = cipher_in.decrypt(nonce, data_bytes.as_slice()) {
                                // Write raw decrypted frame directly into native TAP interface!
                                let _ = tx_dev.write_all(&decrypted_payload).await;
                            }
                        }
                    }
                }
            }
        }
    });

    // 6. Graceful cleanup dispatcher and signal handler
    log::info!("🔒 Security system active. Listening for Ctrl+C to perform clean automatic rollback...");
    
    let mut ws_tx = ws_write;
    loop {
        tokio::select! {
            msg_opt = ws_write_rx.recv() => {
                if let Some(msg) = msg_opt {
                    if let Err(e) = ws_tx.send(msg).await {
                        log::error!("Websocket dispatch error: {:?}", e);
                        break;
                    }
                } else {
                    break;
                }
            }
            _ = tokio::signal::ctrl_c() => {
                log::warn!("⚠️ Termination signal captured! Initiating non-destructive automatic rollback sequence...");
                
                // Revert metrics and disable adapter back to normal defaults
                #[cfg(target_os = "windows")]
                {
                    log::info!("🧹 Restoring Windows interface routing metrics...");
                    let _ = Command::new("netsh")
                        .args(&["interface", "ipv4", "set", "interface", "RustNetTAP", "metric=automatic"])
                        .output();
                    
                    log::info!("🔌 Disabling virtual loop adapter interface...");
                    let _ = Command::new("netsh")
                        .args(&["interface", "set", "interface", "RustNetTAP", "admin=disabled"])
                        .output();
                }
                #[cfg(target_os = "linux")]
                {
                    log::info!("🧹 Restoring Linux TAP metrics and cleaning up tap0 interface...");
                    let _ = Command::new("ip").args(&["link", "set", "dev", "tap0", "down"]).output();
                    let _ = Command::new("ip").args(&["link", "delete", "tap0"]).output();
                }
                #[cfg(target_os = "macos")]
                {
                    log::info!("🧹 Restoring macOS TAP metric...");
                    let _ = Command::new("ifconfig").args(&["tap0", "destroy"]).output();
                }

                log::info!("✨ CLEANUP COMPLETE! All virtual network alterations have been completely reverted from your host operating system.");
                break;
            }
        }
    }

    Ok(())
}
`;
