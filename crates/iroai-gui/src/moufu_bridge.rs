use moufu_adapter_sdk::MoufuClient;
use moufu_protocol::{AppCapabilities, EntityId, SharedMemoryDescriptor};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub enum MoufuGuiCommand {
    RegisterDocument {
        entity_id: EntityId,
        name: String,
        width: u32,
        height: u32,
    },
    NotifyChangeInline {
        entity_id: EntityId,
        version: u64,
        is_transient: bool,
        payload: serde_json::Value,
    },
    NotifyChangeShm {
        entity_id: EntityId,
        version: u64,
        is_transient: bool,
        shm: SharedMemoryDescriptor,
    },
}

/// GUI Bridge that manages communication with Moufu Integration Hub asynchronously
/// without ever blocking the UI frame rendering.
pub struct MoufuBridge {
    cmd_tx: Sender<MoufuGuiCommand>,
    connected: Arc<AtomicBool>,
}

impl Default for MoufuBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl MoufuBridge {
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx): (Sender<MoufuGuiCommand>, Receiver<MoufuGuiCommand>) = mpsc::channel();
        let connected = Arc::new(AtomicBool::new(false));
        let connected_clone = Arc::clone(&connected);

        let endpoint = std::env::var("MOUFU_ENDPOINT").unwrap_or_else(|_| "127.0.0.1:9478".to_string());

        // Spawn a dedicated Tokio runtime thread for async Moufu communication
        thread::Builder::new()
            .name("iroai-moufu-bridge".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        log::error!("Failed to create tokio runtime for Moufu bridge: {e}");
                        return;
                    }
                };

                rt.block_on(async move {
                    let mut client_opt: Option<MoufuClient> = None;

                    loop {
                        // 1. Try to connect if not connected
                        if client_opt.as_ref().is_some_and(|c| c.is_connected()) {
                            connected_clone.store(true, Ordering::Release);
                        } else {
                            connected_clone.store(false, Ordering::Release);
                            let capabilities = AppCapabilities {
                                supports_live_link: true,
                                supports_push_edits: false,
                                supports_delta_sync: true,
                                supports_link_restoration: true,
                                supported_data_types: vec![
                                    "application/x-moufu-raster".to_string(),
                                    "image/png".to_string(),
                                    "image/raw-rgba8".to_string(),
                                ],
                            };

                            match MoufuClient::connect(&endpoint, "Iroai", "0.1.0", capabilities).await {
                                Ok(client) => {
                                    log::info!("Connected to Moufu Hub at {}", endpoint);
                                    connected_clone.store(true, Ordering::Release);
                                    client_opt = Some(client);
                                }
                                Err(_) => {
                                    client_opt = None;
                                    connected_clone.store(false, Ordering::Release);
                                }
                            }
                        }

                        // 2. Process queued GUI commands
                        // Drain up to 20 messages per cycle or wait briefly
                        let mut processed = 0;
                        while let Ok(cmd) = cmd_rx.try_recv() {
                            processed += 1;
                            if let Some(client) = &client_opt {
                                if client.is_connected() {
                                    match cmd {
                                        MoufuGuiCommand::RegisterDocument { entity_id, name, width, height } => {
                                            let mut metadata = HashMap::new();
                                            metadata.insert("width".to_string(), width.to_string());
                                            metadata.insert("height".to_string(), height.to_string());
                                            let _ = client.register_entity(
                                                entity_id,
                                                name,
                                                "application/x-moufu-raster".to_string(),
                                                "Iroai".to_string(),
                                                metadata,
                                            );
                                        }
                                        MoufuGuiCommand::NotifyChangeInline { entity_id, version, is_transient, payload } => {
                                            let _ = client.notify_change_inline(entity_id, version, is_transient, payload);
                                        }
                                        MoufuGuiCommand::NotifyChangeShm { entity_id, version, is_transient, shm } => {
                                            let _ = client.notify_change_shm(entity_id, version, is_transient, shm);
                                        }
                                    }
                                }
                            }
                            if processed >= 20 {
                                break;
                            }
                        }

                        // Sleep interval: faster if connected, slower backoff if offline
                        let sleep_ms = if client_opt.as_ref().is_some_and(|c| c.is_connected()) {
                            50
                        } else {
                            1000
                        };
                        tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
                    }
                });
            })
            .ok();

        Self {
            cmd_tx,
            connected,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Acquire)
    }

    pub fn try_connect(&self) -> bool {
        self.is_connected()
    }

    pub fn register_document(&self, entity_id: &str, name: &str, width: u32, height: u32) {
        let _ = self.cmd_tx.send(MoufuGuiCommand::RegisterDocument {
            entity_id: EntityId::from(entity_id),
            name: name.to_string(),
            width,
            height,
        });
    }

    pub fn notify_canvas_change(&self, entity_id: &str, version: u64, is_transient: bool, layer_count: usize) {
        let payload = serde_json::json!({
            "layer_count": layer_count,
            "status": if is_transient { "in_progress" } else { "committed" },
            "updated_at": chrono::Utc::now().to_rfc3339(),
        });
        let _ = self.cmd_tx.send(MoufuGuiCommand::NotifyChangeInline {
            entity_id: EntityId::from(entity_id),
            version,
            is_transient,
            payload,
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn broadcast_layer_sync(
        &self,
        doc_id: &str,
        layer_id: &str,
        width: u32,
        height: u32,
        blend_mode: &str,
        opacity: f32,
        data: &[u8],
    ) {
        let entity_id = format!("iroai://{doc_id}/layer/{layer_id}");
        let payload = serde_json::json!({
            "action": "layer_sync",
            "document_id": doc_id,
            "layer_id": layer_id,
            "width": width,
            "height": height,
            "blend_mode": blend_mode,
            "opacity": opacity,
            "byte_count": data.len(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        let _ = self.cmd_tx.send(MoufuGuiCommand::NotifyChangeInline {
            entity_id: EntityId::from(entity_id.as_str()),
            version: 1,
            is_transient: false,
            payload,
        });
    }

    pub fn export_asset_to(&self, target_app: &str, asset_type: &str, file_path: &str) {
        let entity_id = format!("iroai://export/{target_app}");
        let payload = serde_json::json!({
            "action": "export_asset",
            "target_app": target_app,
            "asset_type": asset_type,
            "file_path": file_path,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        let _ = self.cmd_tx.send(MoufuGuiCommand::NotifyChangeInline {
            entity_id: EntityId::from(entity_id.as_str()),
            version: 1,
            is_transient: false,
            payload,
        });
    }

    pub fn invalidate_render_cache(&self, doc_id: &str, x: u32, y: u32, width: u32, height: u32) {
        let entity_id = format!("iroai://{doc_id}/cache_invalidation");
        let payload = serde_json::json!({
            "action": "invalidate_render_cache",
            "document_id": doc_id,
            "region": [x, y, width, height],
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        let _ = self.cmd_tx.send(MoufuGuiCommand::NotifyChangeInline {
            entity_id: EntityId::from(entity_id.as_str()),
            version: 1,
            is_transient: true,
            payload,
        });
    }
}
