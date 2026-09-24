use serde::{Deserialize, Serialize};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MoufuMessage {
    RegisterApp {
        name: String,
        version: String,
        capabilities: Vec<String>,
    },
    ExportAsset {
        target_app: String,
        asset_type: String,
        file_path: String,
    },
    NotifyDocumentChanged {
        document_id: String,
        title: String,
    },
    ExecuteBatch {
        config_json: String,
    },
    RunAction {
        action_json: String,
        target_path: String,
    },
    SyncIccProfile {
        document_id: String,
        profile_name: String,
        color_space: String,
        icc_raw_base64: String,
    },
    StreamTileAsset {
        document_id: String,
        tile_x: u32,
        tile_y: u32,
        byte_length: usize,
        checksum: String,
    },
}

/// Zero-latency background asynchronous IPC client
pub struct MoufuClient {
    pub is_connected: bool,
    pub endpoint: String,
    sender: Option<Sender<MoufuMessage>>,
    connected_flag: Arc<Mutex<bool>>,
}

impl Default for MoufuClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MoufuClient {
    pub fn new() -> Self {
        let (tx, rx): (Sender<MoufuMessage>, Receiver<MoufuMessage>) = mpsc::channel();
        let connected_flag = Arc::new(Mutex::new(false));
        let flag_clone = Arc::clone(&connected_flag);
        let endpoint = "127.0.0.1:49200".to_string();
        let ep_clone = endpoint.clone();

        // Spawn dedicated background worker thread so the UI thread NEVER blocks
        thread::Builder::new()
            .name("moufu-ipc-worker".into())
            .spawn(move || {
                let mut active_stream: Option<std::net::TcpStream> = None;

                while let Ok(msg) = rx.recv() {
                    // Try to send or reconnect in background
                    let json = match serde_json::to_string(&msg) {
                        Ok(j) => j,
                        Err(_) => continue,
                    };

                    use std::io::Write;
                    let mut sent = false;

                    if let Some(ref mut s) = active_stream {
                        if writeln!(s, "{}", json).is_ok() {
                            sent = true;
                        } else {
                            active_stream = None;
                        }
                    }

                    if !sent {
                        if let Ok(mut stream) = std::net::TcpStream::connect_timeout(
                            &ep_clone.parse().unwrap_or_else(|_| "127.0.0.1:49200".parse().unwrap()),
                            Duration::from_millis(50),
                        ) {
                            let _ = writeln!(stream, "{}", json);
                            active_stream = Some(stream);
                            if let Ok(mut f) = flag_clone.lock() {
                                *f = true;
                            }
                        } else if let Ok(mut f) = flag_clone.lock() {
                            *f = false;
                        }
                    }
                }
            })
            .ok();

        Self {
            is_connected: false,
            endpoint,
            sender: Some(tx),
            connected_flag,
        }
    }

    pub fn try_connect(&mut self) -> bool {
        if let Ok(f) = self.connected_flag.lock() {
            self.is_connected = *f;
        }
        self.is_connected
    }

    pub fn notify_document_change(&self, doc_id: &str, title: &str) {
        let msg = MoufuMessage::NotifyDocumentChanged {
            document_id: doc_id.to_string(),
            title: title.to_string(),
        };
        let _ = self.send_message(msg);
    }

    pub fn broadcast_icc_profile(&self, doc_id: &str, name: &str, color_space: &str, raw_bytes: &[u8]) {
        let hex = raw_bytes.iter().map(|b| format!("{:02x}", b)).collect();
        let msg = MoufuMessage::SyncIccProfile {
            document_id: doc_id.to_string(),
            profile_name: name.to_string(),
            color_space: color_space.to_string(),
            icc_raw_base64: hex,
        };
        let _ = self.send_message(msg);
    }

    pub fn broadcast_streamed_tile(&self, doc_id: &str, tx: u32, ty: u32, bytes_len: usize) {
        let msg = MoufuMessage::StreamTileAsset {
            document_id: doc_id.to_string(),
            tile_x: tx,
            tile_y: ty,
            byte_length: bytes_len,
            checksum: format!("{:08x}", bytes_len ^ ((tx as usize) << 16) ^ (ty as usize)),
        };
        let _ = self.send_message(msg);
    }

    /// Completely non-blocking dispatch to background channel
    pub fn send_message(&self, message: MoufuMessage) -> Result<(), String> {
        if let Some(tx) = &self.sender {
            tx.send(message).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
