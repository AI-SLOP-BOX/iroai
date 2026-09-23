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
