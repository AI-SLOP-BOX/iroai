use iroai_core::{
    ActionCommand, ActionSequence, BatchJobConfig, BatchProcessor, ExportFormat, ImageIo,
    PhotoAdjustments, PsdHandler,
};
use std::path::PathBuf;

fn print_usage() {
    println!("Iroai CLI v0.1.0 — High Precision Image Processing Engine");
    println!("Usage:");
    println!("  iroai convert <input> <output>              Convert between image formats (PNG, JPEG, WebP, TIFF, PSD)");
    println!("  iroai resize <input> <output> <w> <h>       Resize image with bilinear sampling");
    println!("  iroai crop <input> <output> <x> <y> <w> <h> Crop image region");
    println!("  iroai filter <input> <output> <blur|invert|grayscale|sharpen> Apply filter to image");
    println!("  iroai photo <input> <output> <ev> <temp>    Adjust exposure (EV) and color temp");
    println!("  iroai batch <config.json>                   Run automated batch job defined in JSON");
    println!("  iroai verify <project.iroai>                Verify project integrity and layer structure");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "convert" => {
            if args.len() < 4 {
                eprintln!("Error: 'convert' requires <input> and <output>");
                return;
            }
            let in_path = PathBuf::from(&args[2]);
            let out_path = PathBuf::from(&args[3]);

            println!("Reading: {:?}", in_path);
            let doc = if in_path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("psd")).unwrap_or(false) {
                PsdHandler::load_psd(&in_path).expect("Failed to load PSD")
            } else {
                ImageIo::load_image(&in_path).expect("Failed to load image")
            };

            let out_ext = out_path.extension().and_then(|e| e.to_str()).unwrap_or("png").to_lowercase();
            match out_ext.as_str() {
                "psd" => PsdHandler::export_psd(&doc, &out_path).expect("Failed to export PSD"),
                "jpg" | "jpeg" => ImageIo::export_jpeg(&doc, &out_path, 90).expect("Failed to export JPEG"),
                "webp" => ImageIo::export_webp(&doc, &out_path).expect("Failed to export WebP"),
                "tif" | "tiff" => ImageIo::export_tiff(&doc, &out_path).expect("Failed to export TIFF"),
                _ => ImageIo::export_png(&doc, &out_path).expect("Failed to export PNG"),
            }
            println!("Successfully converted to {:?}", out_path);
        }
        "resize" => {
            if args.len() < 6 {
                eprintln!("Error: 'resize' requires <input> <output> <width> <height>");
                return;
            }
            let in_path = PathBuf::from(&args[2]);
            let out_path = PathBuf::from(&args[3]);
            let w: u32 = args[4].parse().expect("Invalid width");
            let h: u32 = args[5].parse().expect("Invalid height");

            let mut doc = ImageIo::load_image(&in_path).expect("Failed to load image");
            let cmd = ActionCommand::Resize { width: w, height: h };
            cmd.execute(&mut doc).expect("Failed to execute resize");
            ImageIo::export_png(&doc, &out_path).expect("Failed to export PNG");
            println!("Resized to {}x{} -> {:?}", w, h, out_path);
        }
        "crop" => {
            if args.len() < 8 {
                eprintln!("Error: 'crop' requires <input> <output> <x> <y> <width> <height>");
                return;
            }
            let in_path = PathBuf::from(&args[2]);
            let out_path = PathBuf::from(&args[3]);
            let x: u32 = args[4].parse().expect("Invalid x");
            let y: u32 = args[5].parse().expect("Invalid y");
            let w: u32 = args[6].parse().expect("Invalid width");
            let h: u32 = args[7].parse().expect("Invalid height");

            let mut doc = ImageIo::load_image(&in_path).expect("Failed to load image");
            let cmd = ActionCommand::Crop { x, y, width: w, height: h };
            cmd.execute(&mut doc).expect("Failed to execute crop");
            ImageIo::export_png(&doc, &out_path).expect("Failed to export PNG");
            println!("Cropped to {}x{} -> {:?}", w, h, out_path);
        }
        "filter" => {
            if args.len() < 5 {
                eprintln!("Error: 'filter' requires <input> <output> <blur|invert|grayscale|sharpen>");
                return;
            }
            let in_path = PathBuf::from(&args[2]);
            let out_path = PathBuf::from(&args[3]);
            let filter_name = &args[4];

            let mut doc = ImageIo::load_image(&in_path).expect("Failed to load image");
            let cmd = match filter_name.as_str() {
                "blur" => ActionCommand::ApplyGaussianBlur { radius: 5 },
                "sharpen" => ActionCommand::ApplySharpen { strength: 1.5 },
                "invert" => ActionCommand::ApplyInvert,
                "grayscale" => ActionCommand::ApplyGrayscale,
                _ => panic!("Unknown filter: {}", filter_name),
            };
            cmd.execute(&mut doc).expect("Failed to execute filter");
            ImageIo::export_png(&doc, &out_path).expect("Failed to export PNG");
            println!("Filter '{}' applied -> {:?}", filter_name, out_path);
        }
        "photo" => {
            if args.len() < 6 {
                eprintln!("Error: 'photo' requires <input> <output> <ev> <temp>");
                return;
            }
            let in_path = PathBuf::from(&args[2]);
            let out_path = PathBuf::from(&args[3]);
            let ev: f32 = args[4].parse().expect("Invalid EV");
            let temp: f32 = args[5].parse().expect("Invalid temperature");

            let mut doc = ImageIo::load_image(&in_path).expect("Failed to load image");
            let mut adj = PhotoAdjustments::default();
            adj.exposure = ev;
            adj.temperature = temp;
            let cmd = ActionCommand::ApplyPhotoAdjustments(adj);
            cmd.execute(&mut doc).expect("Failed to execute photo adjustments");
            ImageIo::export_png(&doc, &out_path).expect("Failed to export PNG");
            println!("Photo adjustments applied (EV: {}, Temp: {}) -> {:?}", ev, temp, out_path);
        }
        "verify" => {
            if args.len() < 3 {
                eprintln!("Error: 'verify' requires <project.iroai>");
                return;
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
                    eprintln!("Project Verification FAILED: {}", err);
                    std::process::exit(1);
                }
            }
        }
        "batch" => {
            if args.len() < 3 {
                eprintln!("Error: 'batch' requires <config.json>");
                return;
            }
            let config_path = PathBuf::from(&args[2]);
            let config_str = std::fs::read_to_string(&config_path).expect("Failed to read batch config file");
            let config: BatchJobConfig = serde_json::from_str(&config_str).expect("Failed to parse BatchJobConfig JSON");

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
        }
        cmd => {
            eprintln!("Unknown command: {}", cmd);
            print_usage();
        }
    }
}
