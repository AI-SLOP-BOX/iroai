use serde::{Deserialize, Serialize};

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
    /// Synchronize ICC Color Profile across creative suite pipeline
    SyncIccProfile {
        document_id: String,
        profile_name: String,
        color_space: String,
        icc_raw_base64: String,
    },
    /// Stream virtual swapped scratch tile or asset metadata to companion apps
    StreamTileAsset {
        document_id: String,
        tile_x: u32,
        tile_y: u32,
        byte_length: usize,
        checksum: String,
    },
}

pub struct MoufuClient {
    pub is_connected: bool,
    pub endpoint: String,
}

impl Default for MoufuClient {
    fn default() -> Self {
        Self {
            is_connected: false,
            endpoint: "127.0.0.1:49200".to_string(),
        }
    }
}

impl MoufuClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn try_connect(&mut self) -> bool {
        if let Ok(_stream) = std::net::TcpStream::connect(&self.endpoint) {
            self.is_connected = true;
            true
        } else {
            self.is_connected = false;
            false
        }
    }

    pub fn notify_document_change(&self, doc_id: &str, title: &str) {
        let msg = MoufuMessage::NotifyDocumentChanged {
            document_id: doc_id.to_string(),
            title: title.to_string(),
        };
        let _ = self.send_message(msg);
    }

    /// Broadcast ICC profile metadata to other tools in the Moufu creative ecosystem
    pub fn broadcast_icc_profile(&self, doc_id: &str, name: &str, color_space: &str, raw_bytes: &[u8]) {
        // Simple hex representation of raw bytes
        let hex = raw_bytes.iter().map(|b| format!("{:02x}", b)).collect();
        let msg = MoufuMessage::SyncIccProfile {
            document_id: doc_id.to_string(),
            profile_name: name.to_string(),
            color_space: color_space.to_string(),
            icc_raw_base64: hex,
        };
        let _ = self.send_message(msg);
    }

    /// Broadcast streamed tile sync event
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

    pub fn send_message(&self, message: MoufuMessage) -> Result<(), String> {
        if !self.is_connected {
            return Ok(());
        }
        if let Ok(mut stream) = std::net::TcpStream::connect(&self.endpoint) {
            use std::io::Write;
            if let Ok(json) = serde_json::to_string(&message) {
                let _ = writeln!(stream, "{}", json);
            }
        }
        Ok(())
    }
}
