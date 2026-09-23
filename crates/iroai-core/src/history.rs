use crate::buffer::{PixelBuffer, TILE_SIZE};
use crate::layer::LayerId;

#[derive(Debug, Clone)]
pub struct TilePatch {
    pub layer_id: LayerId,
    pub tile_x: u32,
    pub tile_y: u32,
    pub before_data: Vec<u8>,
    pub after_data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub enum HistoryAction {
    PixelChange {
        description: String,
        patches: Vec<TilePatch>,
    },
    LayerCreated {
        index: usize,
        id: LayerId,
        name: String,
        width: u32,
        height: u32,
        kind: crate::layer::LayerKind,
    },
    LayerAdded {
        index: usize,
        layer: Box<crate::layer::Layer>,
    },
    LayerRemoved {
        index: usize,
        layer: Box<crate::layer::Layer>,
    },
    LayerReordered {
        from: usize,
        to: usize,
    },
}

impl HistoryAction {
    pub fn estimated_bytes(&self) -> usize {
        match self {
            HistoryAction::PixelChange { patches, .. } => {
                patches.iter().map(|p| p.before_data.len() + p.after_data.len()).sum()
            }
            HistoryAction::LayerCreated { .. } => 128,
            HistoryAction::LayerAdded { layer, .. } | HistoryAction::LayerRemoved { layer, .. } => {
                layer.buffer.data.len()
            }
            HistoryAction::LayerReordered { .. } => 64,
        }
    }
}

#[derive(Debug)]
pub struct HistoryManager {
    undo_stack: Vec<HistoryAction>,
    redo_stack: Vec<HistoryAction>,
    max_history: usize,
    max_memory_bytes: usize, // e.g. 512MB limit
    current_memory_bytes: usize,
}

impl Default for HistoryManager {
    fn default() -> Self {
        Self::new(50)
    }
}

impl HistoryManager {
    pub fn new(max_history: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history: if max_history == 0 { 50 } else { max_history },
            max_memory_bytes: 256 * 1024 * 1024, // 256 MB default ceiling
            current_memory_bytes: 0,
        }
    }

    pub fn with_memory_limit(mut self, bytes: usize) -> Self {
        self.max_memory_bytes = bytes;
        self
    }

    pub fn current_memory_bytes(&self) -> usize {
        self.current_memory_bytes
    }

    pub fn push(&mut self, action: HistoryAction) {
        self.redo_stack.clear();
        let action_bytes = action.estimated_bytes();
        self.current_memory_bytes += action_bytes;
        self.undo_stack.push(action);

        // 世代数オーバーまたはメモリサイズオーバー時の古い履歴の刈り取り
        while (self.undo_stack.len() > self.max_history || self.current_memory_bytes > self.max_memory_bytes)
            && self.undo_stack.len() > 1
        {
            let removed = self.undo_stack.remove(0);
            self.current_memory_bytes = self.current_memory_bytes.saturating_sub(removed.estimated_bytes());
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    pub fn pop_undo(&mut self) -> Option<HistoryAction> {
        let action = self.undo_stack.pop()?;
        self.redo_stack.push(action.clone());
        Some(action)
    }

    pub fn pop_redo(&mut self) -> Option<HistoryAction> {
        let action = self.redo_stack.pop()?;
        self.undo_stack.push(action.clone());
        Some(action)
    }
}

pub struct StrokeRecorder {
    pub layer_id: LayerId,
    pub before_tiles: std::collections::HashMap<(u32, u32), Vec<u8>>,
}

impl StrokeRecorder {
    pub fn new(layer_id: LayerId) -> Self {
        Self {
            layer_id,
            before_tiles: std::collections::HashMap::new(),
        }
    }

    pub fn record_area_before_touch(&mut self, buffer: &PixelBuffer, min_x: u32, min_y: u32, max_x: u32, max_y: u32) {
        let start_tile_x = min_x / (TILE_SIZE as u32);
        let end_tile_x = max_x / (TILE_SIZE as u32);
        let start_tile_y = min_y / (TILE_SIZE as u32);
        let end_tile_y = max_y / (TILE_SIZE as u32);

        for ty in start_tile_y..=end_tile_y {
            for tx in start_tile_x..=end_tile_x {
                if !self.before_tiles.contains_key(&(tx, ty)) {
                    if let Some(tile_data) = buffer.extract_tile(tx, ty) {
                        self.before_tiles.insert((tx, ty), tile_data);
                    }
                }
            }
        }
    }

    pub fn finish(self, buffer: &PixelBuffer, description: impl Into<String>) -> Option<HistoryAction> {
        let mut patches = Vec::new();

        for ((tx, ty), before_data) in self.before_tiles {
            if let Some(after_data) = buffer.extract_tile(tx, ty) {
                if before_data != after_data {
                    patches.push(TilePatch {
                        layer_id: self.layer_id,
                        tile_x: tx,
                        tile_y: ty,
                        before_data,
                        after_data,
                    });
                }
            }
        }

        if patches.is_empty() {
            None
        } else {
            Some(HistoryAction::PixelChange {
                description: description.into(),
                patches,
            })
        }
    }
}
