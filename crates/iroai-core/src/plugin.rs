use crate::buffer::PixelBuffer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginPermission {
    ReadOnly,
    ModifyPixels,
    FileSystemAccess,
    NetworkAccess,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub permissions: Vec<PluginPermission>,
}

pub trait FilterPlugin: Send + Sync {
    fn manifest(&self) -> PluginManifest;
    fn process_pixels(&self, buffer: &mut PixelBuffer, params: &serde_json::Value) -> Result<(), String>;
}

/// 隔離・安全なプラグインレジストリ
pub struct PluginRegistry {
    pub filters: Vec<Box<dyn FilterPlugin>>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self {
            filters: Vec::new(),
        }
    }
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_filter(&mut self, plugin: Box<dyn FilterPlugin>) {
        self.filters.push(plugin);
    }

    pub fn execute_filter(&self, plugin_id: &str, buffer: &mut PixelBuffer, params: &serde_json::Value) -> Result<(), String> {
        if let Some(plugin) = self.filters.iter().find(|p| p.manifest().id == plugin_id) {
            let manifest = plugin.manifest();
            if !manifest.permissions.contains(&PluginPermission::ModifyPixels) {
                return Err("Plugin does not have permission to modify pixels".to_string());
            }
            plugin.process_pixels(buffer, params)
        } else {
            Err(format!("Plugin with id '{}' not found", plugin_id))
        }
    }
}
