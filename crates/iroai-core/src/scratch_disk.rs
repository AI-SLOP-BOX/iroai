//! Scratch Disk (Virtual Memory Paging) Engine for Iroai
//!
//! When canvas sizes or layer counts exceed physical RAM limits, this module
//! transparently evicts LRU (Least Recently Used) 64x64 tiles (16KB) to a temporary
//! scratch file on SSD, swapping them back into RAM seamlessly upon access.

use crate::buffer::TILE_SIZE;
use std::collections::{HashMap, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const TILE_BYTES: usize = TILE_SIZE * TILE_SIZE * 4; // 64 * 64 * 4 = 16,384 bytes (16KB)

static SCRATCH_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Unique identifier for a tile in a document (Layer ID + Tile X + Tile Y).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileKey {
    pub layer_id: u32,
    pub tx: u32,
    pub ty: u32,
}

/// Metadata about a swapped-out tile on disk.
#[derive(Debug, Clone, Copy)]
struct DiskSlot {
    offset: u64,
    size: usize,
}

/// Scratch disk swap manager.
pub struct ScratchDiskManager {
    scratch_path: PathBuf,
    file: File,
    max_ram_tiles: usize,
    ram_cache: HashMap<TileKey, Vec<u8>>,
    lru_order: VecDeque<TileKey>,
    disk_slots: HashMap<TileKey, DiskSlot>,
    next_disk_offset: u64,
    reclaimed_slots: Vec<u64>,
}

impl ScratchDiskManager {
    /// Creates a new ScratchDiskManager with a specified RAM tile capacity and scratch directory.
    pub fn new<P: AsRef<Path>>(scratch_dir: P, max_ram_tiles: usize) -> std::io::Result<Self> {
        let id = SCRATCH_COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        let file_name = format!(".iroai_scratch_{}_{}.tmp", pid, id);
        let scratch_path = scratch_dir.as_ref().join(file_name);

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&scratch_path)?;

        Ok(Self {
            scratch_path,
            file,
            max_ram_tiles: max_ram_tiles.max(4),
            ram_cache: HashMap::new(),
            lru_order: VecDeque::new(),
            disk_slots: HashMap::new(),
            next_disk_offset: 0,
            reclaimed_slots: Vec::new(),
        })
    }

    /// Number of tiles currently residing in physical RAM.
    pub fn ram_tile_count(&self) -> usize {
        self.ram_cache.len()
    }

    /// Number of tiles currently swapped out to disk.
    pub fn disk_tile_count(&self) -> usize {
        self.disk_slots.len()
    }

    /// Returns the active scratch file path on disk.
    pub fn scratch_path(&self) -> &Path {
        &self.scratch_path
    }

    /// Touches a key in the LRU order queue.
    fn touch_lru(&mut self, key: TileKey) {
        if let Some(pos) = self.lru_order.iter().position(|&k| k == key) {
            self.lru_order.remove(pos);
        }
        self.lru_order.push_back(key);
    }

    /// Writes or updates a tile in memory, swapping out LRU tiles if capacity is exceeded.
    pub fn put_tile(&mut self, key: TileKey, tile: Vec<u8>) -> std::io::Result<()> {
        // If tile was on disk, clear its disk reservation since we're writing a new version in RAM
        if let Some(slot) = self.disk_slots.remove(&key) {
            self.reclaimed_slots.push(slot.offset);
        }

        self.ram_cache.insert(key, tile);
        self.touch_lru(key);

        // Evict LRU tiles if exceeding max RAM capacity
        while self.ram_cache.len() > self.max_ram_tiles {
            if let Some(evict_key) = self.lru_order.pop_front() {
                if let Some(evicted_tile) = self.ram_cache.remove(&evict_key) {
                    self.swap_out_to_disk(evict_key, &evicted_tile)?;
                }
            } else {
                break;
            }
        }

        Ok(())
    }

    /// Retrieves a tile, reading from disk if currently swapped out.
    pub fn get_tile(&mut self, key: TileKey) -> std::io::Result<Option<Vec<u8>>> {
        if let Some(tile) = self.ram_cache.get(&key) {
            let cloned = tile.clone();
            self.touch_lru(key);
            return Ok(Some(cloned));
        }

        // Check if swapped out to disk
        if let Some(slot) = self.disk_slots.remove(&key) {
            let mut buf = vec![0u8; slot.size];
            self.file.seek(SeekFrom::Start(slot.offset))?;
            self.file.read_exact(&mut buf)?;
            self.reclaimed_slots.push(slot.offset);

            // Put back into RAM (might trigger another eviction)
            self.put_tile(key, buf.clone())?;
            return Ok(Some(buf));
        }

        Ok(None)
    }

    /// Evicts a single tile from RAM directly to disk.
    fn swap_out_to_disk(&mut self, key: TileKey, tile: &[u8]) -> std::io::Result<()> {
        let offset = if let Some(reclaimed) = self.reclaimed_slots.pop() {
            reclaimed
        } else {
            let off = self.next_disk_offset;
            self.next_disk_offset += TILE_BYTES as u64;
            off
        };

        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write_all(tile)?;
        self.file.flush()?;

        self.disk_slots.insert(
            key,
            DiskSlot {
                offset,
                size: tile.len(),
            },
        );

        Ok(())
    }
}

impl Drop for ScratchDiskManager {
    fn drop(&mut self) {
        // Clean up temporary scratch file on disk
        let _ = std::fs::remove_file(&self.scratch_path);
    }
}
