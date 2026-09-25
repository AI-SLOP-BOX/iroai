use iroai_core::{
    ActionCommand, BatchJobConfig, BatchProcessor, Document, ImageIo,
    PhotoAdjustments, PsdHandler,
};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn print_usage() {
    println!("Iroai CLI v0.1.0 — High Precision Image Processing Engine");
    println!("Usage:");
    println!("  iroai convert <input> <output>              Convert between image formats (PNG, JPEG, WebP, TIFF, PSD, .iroai)");
    println!("  iroai resize <input> <output> <w> <h>       Resize image with bilinear sampling");
    println!("  iroai crop <input> <output> <x> <y> <w> <h> Crop image region");
    println!("  iroai filter <input> <output> <blur|invert|grayscale|sharpen> Apply filter to image");
    println!("  iroai photo <input> <output> <ev> <temp>    Adjust exposure (EV) and color temp");
    println!("  iroai batch <config.json>                   Run automated batch job defined in JSON");
    println!("  iroai verify <project.iroai>                Verify project integrity and layer structure");
}

fn load_document(path: &Path) -> Result<Document, String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    match ext.as_str() {
        "psd" => PsdHandler::load_psd(path).map_err(|e| format!("Failed to load PSD: {e}")),
        "iroai" => ImageIo::load_project(path).map_err(|e| format!("Failed to load .iroai project: {e}")),
        _ => ImageIo::load_image(path).map_err(|e| format!("Failed to load image: {e}")),
    }
}

fn export_document(doc: &Document, path: &Path) -> Result<(), String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("png").to_lowercase();
    match ext.as_str() {
        "psd" => PsdHandler::export_psd(doc, path).map_err(|e| format!("Failed to export PSD: {e}")),
        "jpg" | "jpeg" => ImageIo::export_jpeg(doc, path, 90).map_err(|e| format!("Failed to export JPEG: {e}")),
        "webp" => ImageIo::export_webp(doc, path).map_err(|e| format!("Failed to export WebP: {e}")),
        "tif" | "tiff" => ImageIo::export_tiff(doc, path).map_err(|e| format!("Failed to export TIFF: {e}")),
        "iroai" => ImageIo::save_project(doc, path).map_err(|e| format!("Failed to save .iroai project: {e}")),
        _ => ImageIo::export_png(doc, path).map_err(|e| format!("Failed to export PNG: {e}")),
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    match args[1].as_str() {
        "convert" => {
            if args.len() < 4 {
                return Err("Usage: iroai convert <input> <output>".into());
            }
            let in_path = PathBuf::from(&args[2]);
            let out_path = PathBuf::from(&args[3]);

            println!("Reading: {:?}", in_path);
            let doc = load_document(&in_path)?;
            export_document(&doc, &out_path)?;
            println!("Successfully converted to {:?}", out_path);
        }
        "resize" => {
            if args.len() < 6 {
                return Err("Usage: iroai resize <input> <output> <width> <height>".into());
            }
            let in_path = PathBuf::from(&args[2]);
            let out_path = PathBuf::from(&args[3]);
            let w: u32 = args[4].parse().map_err(|_| format!("Invalid width: '{}'", args[4]))?;
            let h: u32 = args[5].parse().map_err(|_| format!("Invalid height: '{}'", args[5]))?;

            let mut doc = load_document(&in_path)?;
            let cmd = ActionCommand::Resize { width: w, height: h };
            cmd.execute(&mut doc).map_err(|e| format!("Failed to execute resize: {e}"))?;
            export_document(&doc, &out_path)?;
            println!("Resized to {}x{} -> {:?}", w, h, out_path);
        }
        "crop" => {
            if args.len() < 8 {
                return Err("Usage: iroai crop <input> <output> <x> <y> <width> <height>".into());
            }
            let in_path = PathBuf::from(&args[2]);
            let out_path = PathBuf::from(&args[3]);
            let x: u32 = args[4].parse().map_err(|_| format!("Invalid x: '{}'", args[4]))?;
            let y: u32 = args[5].parse().map_err(|_| format!("Invalid y: '{}'", args[5]))?;
            let w: u32 = args[6].parse().map_err(|_| format!("Invalid width: '{}'", args[6]))?;
            let h: u32 = args[7].parse().map_err(|_| format!("Invalid height: '{}'", args[7]))?;

            let mut doc = load_document(&in_path)?;
            let cmd = ActionCommand::Crop { x, y, width: w, height: h };
            cmd.execute(&mut doc).map_err(|e| format!("Failed to execute crop: {e}"))?;
            export_document(&doc, &out_path)?;
            println!("Cropped to {}x{} -> {:?}", w, h, out_path);
        }
        "filter" => {
            if args.len() < 5 {
                return Err("Usage: iroai filter <input> <output> <blur|invert|grayscale|sharpen>".into());
            }
            let in_path = PathBuf::from(&args[2]);
            let out_path = PathBuf::from(&args[3]);
            let filter_name = &args[4];

            let mut doc = load_document(&in_path)?;
            let cmd = match filter_name.as_str() {
                "blur" => ActionCommand::ApplyGaussianBlur { radius: 5 },
                "sharpen" => ActionCommand::ApplySharpen { strength: 1.5 },
                "invert" => ActionCommand::ApplyInvert,
                "grayscale" => ActionCommand::ApplyGrayscale,
                unknown => return Err(format!("Unknown filter '{}'. Expected one of: blur, invert, grayscale, sharpen", unknown)),
            };
            cmd.execute(&mut doc).map_err(|e| format!("Failed to execute filter: {e}"))?;
            export_document(&doc, &out_path)?;
            println!("Filter '{}' applied -> {:?}", filter_name, out_path);
        }
        "photo" => {
            if args.len() < 6 {
                return Err("Usage: iroai photo <input> <output> <ev> <temp>".into());
            }
            let in_path = PathBuf::from(&args[2]);
            let out_path = PathBuf::from(&args[3]);
            let ev: f32 = args[4].parse().map_err(|_| format!("Invalid exposure (EV): '{}'", args[4]))?;
            let temp: f32 = args[5].parse().map_err(|_| format!("Invalid color temp: '{}'", args[5]))?;

            let mut doc = load_document(&in_path)?;
            let adj = PhotoAdjustments {
                exposure: ev,
                temperature: temp,
                ..PhotoAdjustments::default()
            };
            let cmd = ActionCommand::ApplyPhotoAdjustments(adj);
            cmd.execute(&mut doc).map_err(|e| format!("Failed to execute photo adjustments: {e}"))?;
            export_document(&doc, &out_path)?;
            println!("Photo adjustments applied (EV: {}, Temp: {}) -> {:?}", ev, temp, out_path);
        }
        "verify" => {
            if args.len() < 3 {
                return Err("Usage: iroai verify <project.iroai>".into());
            }
            let proj_path = PathBuf::from(&args[2]);
            match ImageIo::load_project(&proj_path) {
                Ok(doc) => {
                    println!("Project Verification SUCCESS: {:?}", proj_path);
                    println!("  Title: {}", doc.title);
                    println!("  Dimensions: {}x{} px", doc.width, doc.height);
                    println!("  Layers: {}", doc.layers.len());
                    for (i, layer) in doc.layers.iter().enumerate() {
                        println!("    [{}] \"{}\" (Visible: {}, Opacity: {:.2}, Blend: {:?})", i, layer.name, layer.visible, layer.opacity, layer.blend_mode);
                    }
                }
                Err(err) => {
                    return Err(format!("Project Verification FAILED: {err}"));
                }
            }
        }
        "batch" => {
            if args.len() < 3 {
                return Err("Usage: iroai batch <config.json>".into());
            }
            let config_path = PathBuf::from(&args[2]);
            let config_str = std::fs::read_to_string(&config_path)
                .map_err(|e| format!("Failed to read batch config file {:?}: {e}", config_path))?;
            let config: BatchJobConfig = serde_json::from_str(&config_str)
                .map_err(|e| format!("Failed to parse BatchJobConfig JSON: {e}"))?;

            println!("Starting Batch Job: {} files to {:?}", config.input_files.len(), config.output_directory);
            let progress = BatchProcessor::process_batch(&config, |p| {
                println!("  [{}/{}] Processing: {}", p.current_index, p.total_files, p.current_filename);
                true
            });

            println!("Batch Completed: {} succeeded, {} failed", progress.succeeded_count, progress.failed_count);
            if !progress.errors.is_empty() {
                println!("Errors:");
                for e in &progress.errors {
                    println!("  - {}", e);
                }
            }
            if progress.failed_count > 0 {
                return Err(format!("Batch processing completed with {} failure(s)", progress.failed_count));
            }
        }
        cmd => {
            print_usage();
            return Err(format!("Unknown command: '{}'", cmd));
        }
    }

    Ok(())
}

fn main() -> ExitCode {
    if let Err(err) = run() {
        eprintln!("Error: {}", err);
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
