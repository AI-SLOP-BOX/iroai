use crate::document::Document;
use crate::io::{ImageIo, IoError};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    pub auto_save_interval_secs: u64,
    pub max_generations: usize,
    pub backup_dir: PathBuf,
}

impl Default for BackupConfig {
    fn default() -> Self {
        let default_backup = std::env::temp_dir().join("iroai_backups");
        Self {
            auto_save_interval_secs: 180, // 3 minutes
            max_generations: 5,
            backup_dir: default_backup,
        }
    }
}

pub struct RecoveryManager {
    pub config: BackupConfig,
    pub last_save_time: std::time::Instant,
}

impl RecoveryManager {
    pub fn new(config: BackupConfig) -> Self {
        let _ = std::fs::create_dir_all(&config.backup_dir);
        Self {
            config,
            last_save_time: std::time::Instant::now(),
        }
    }

    /// 自動保存判定
    pub fn should_auto_save(&self) -> bool {
        self.last_save_time.elapsed().as_secs() >= self.config.auto_save_interval_secs
    }

    /// 世代管理された自動バックアップの実行
    pub fn create_auto_backup(&mut self, doc: &Document) -> Result<PathBuf, IoError> {
        let _ = std::fs::create_dir_all(&self.config.backup_dir);

        // タイムスタンプ付きバックアップ名
        let timestamp = chrono_like_timestamp();
        let backup_file = self.config.backup_dir.join(format!("{}_auto_{}.iroai", doc.title, timestamp));

        ImageIo::save_project(doc, &backup_file)?;
        self.last_save_time = std::time::Instant::now();

        // 世代ローテーション (古いバックアップの削除)
        self.prune_old_backups(&doc.title);

        Ok(backup_file)
    }

    /// クラッシュ未保存ファイルの検出
    pub fn find_recoverable_backups(&self, title: &str) -> Vec<PathBuf> {
        let mut backups = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.config.backup_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                    if file_name.starts_with(title) && file_name.ends_with(".iroai") {
                        backups.push(path);
                    }
                }
            }
        }
        backups.sort_by(|a, b| b.cmp(a)); // Newest first
        backups
    }

    fn prune_old_backups(&self, title: &str) {
        let backups = self.find_recoverable_backups(title);
        if backups.len() > self.config.max_generations {
            for old in &backups[self.config.max_generations..] {
                let _ = std::fs::remove_file(old);
            }
        }
    }
}

fn chrono_like_timestamp() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}
