use crate::action::ActionSequence;
use crate::io::ImageIo;
use crate::psd::PsdHandler;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    Png,
    Jpeg { quality: u8 },
    WebP,
    Tiff,
    Psd,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchJobConfig {
    pub input_files: Vec<PathBuf>,
    pub output_directory: PathBuf,
    pub output_format: ExportFormat,
    pub filename_prefix: String,
    pub filename_suffix: String,
    pub sequential_numbering: bool,
    pub continue_on_error: bool,
    pub actions: ActionSequence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProgress {
    pub current_index: usize,
    pub total_files: usize,
    pub current_filename: String,
    pub succeeded_count: usize,
    pub failed_count: usize,
    pub errors: Vec<String>,
}

pub struct BatchProcessor;

impl BatchProcessor {
    pub fn process_batch<F>(config: &BatchJobConfig, mut on_progress: F) -> BatchProgress
    where
        F: FnMut(&BatchProgress) -> bool, // Return false to cancel
    {
        let total = config.input_files.len();
        let mut progress = BatchProgress {
            current_index: 0,
            total_files: total,
            current_filename: String::new(),
            succeeded_count: 0,
            failed_count: 0,
            errors: Vec::new(),
        };

        let _ = std::fs::create_dir_all(&config.output_directory);

        for (i, in_path) in config.input_files.iter().enumerate() {
            progress.current_index = i + 1;
            let filename = in_path.file_stem().and_then(|s| s.to_str()).unwrap_or("image");
            progress.current_filename = in_path.to_string_lossy().to_string();

            // Progress callback check for cancellation
            if !on_progress(&progress) {
                progress.errors.push("Batch processing cancelled by user".to_string());
                break;
            }

            let result = (|| -> Result<(), String> {
                // 1. 画像読み込み
                let mut doc = if in_path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("psd")).unwrap_or(false) {
                    PsdHandler::load_psd(in_path).map_err(|e| format!("Failed to read PSD: {}", e))?
                } else {
                    ImageIo::load_image(in_path).map_err(|e| format!("Failed to read image: {}", e))?
                };

                // 2. アクションの実行
                config.actions.execute_all(&mut doc)?;

                // 3. 出力ファイル名の生成
                let out_filename = if config.sequential_numbering {
                    format!("{}{}{:04}{}", config.filename_prefix, filename, i + 1, config.filename_suffix)
                } else {
                    format!("{}{}{}", config.filename_prefix, filename, config.filename_suffix)
                };

                let ext = match config.output_format {
                    ExportFormat::Png => "png",
                    ExportFormat::Jpeg { .. } => "jpg",
                    ExportFormat::WebP => "webp",
                    ExportFormat::Tiff => "tif",
                    ExportFormat::Psd => "psd",
                };

                let out_path = config.output_directory.join(format!("{}.{}", out_filename, ext));

                // 4. 書き出し
                match config.output_format {
                    ExportFormat::Png => ImageIo::export_png(&doc, &out_path).map_err(|e| e.to_string())?,
                    ExportFormat::Jpeg { quality } => ImageIo::export_jpeg(&doc, &out_path, quality).map_err(|e| e.to_string())?,
                    ExportFormat::WebP => ImageIo::export_webp(&doc, &out_path).map_err(|e| e.to_string())?,
                    ExportFormat::Tiff => ImageIo::export_tiff(&doc, &out_path).map_err(|e| e.to_string())?,
                    ExportFormat::Psd => PsdHandler::export_psd(&doc, &out_path).map_err(|e| e.to_string())?,
                }

                Ok(())
            })();

            match result {
                Ok(()) => progress.succeeded_count += 1,
                Err(err) => {
                    progress.failed_count += 1;
                    progress.errors.push(format!("{}: {}", filename, err));
                    if !config.continue_on_error {
                        break;
                    }
                }
            }
        }

        progress
    }
}
