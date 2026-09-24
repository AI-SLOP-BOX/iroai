use crate::buffer::PixelBuffer;
use crate::document::Document;
use crate::layer::Layer;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IoError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Invalid project file: {0}")]
    InvalidFormat(String),
}

pub struct ImageIo;

impl ImageIo {
    pub fn load_image<P: AsRef<Path>>(path: P) -> Result<Document, IoError> {
        let img = image::open(&path)?;
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        let data = rgba.into_raw();

        let buffer = PixelBuffer::from_raw(w, h, data).map_err(|e| IoError::InvalidFormat(e))?;
        let filename = path.as_ref().file_stem().and_then(|s| s.to_str()).unwrap_or("Untitled");
        let layer = Layer::new_from_buffer("Background", buffer);
        let doc = Document::new_with_layer(layer, filename);
        Ok(doc)
    }

    pub fn export_png<P: AsRef<Path>>(doc: &Document, path: P) -> Result<(), IoError> {
        let composite = doc.composite();
        let img_buf = image::RgbaImage::from_raw(composite.width, composite.height, composite.data)
            .ok_or_else(|| IoError::InvalidFormat("Failed to construct RgbaImage from buffer".into()))?;
        img_buf.save_with_format(path, image::ImageFormat::Png)?;
        Ok(())
    }

    pub fn export_jpeg<P: AsRef<Path>>(doc: &Document, path: P, quality: u8) -> Result<(), IoError> {
        let composite = doc.composite();
        let rgb_data: Vec<u8> = composite.data.chunks_exact(4).flat_map(|px| [px[0], px[1], px[2]]).collect();
        let img_buf = image::RgbImage::from_raw(composite.width, composite.height, rgb_data)
            .ok_or_else(|| IoError::InvalidFormat("Failed to construct RgbImage from buffer".into()))?;
        let mut file = File::create(path)?;
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, quality);
        encoder.encode(img_buf.as_raw(), composite.width, composite.height, image::ExtendedColorType::Rgb8)?;
        Ok(())
    }

    pub fn export_webp<P: AsRef<Path>>(doc: &Document, path: P) -> Result<(), IoError> {
        let composite = doc.composite();
        let img_buf = image::RgbaImage::from_raw(composite.width, composite.height, composite.data)
            .ok_or_else(|| IoError::InvalidFormat("Failed to construct RgbaImage from buffer".into()))?;
        img_buf.save_with_format(path, image::ImageFormat::WebP)?;
        Ok(())
    }

    pub fn export_tiff<P: AsRef<Path>>(doc: &Document, path: P) -> Result<(), IoError> {
        let composite = doc.composite();
        let img_buf = image::RgbaImage::from_raw(composite.width, composite.height, composite.data)
            .ok_or_else(|| IoError::InvalidFormat("Failed to construct RgbaImage from buffer".into()))?;
        img_buf.save_with_format(path, image::ImageFormat::Tiff)?;
        Ok(())
    }

    pub fn save_project<P: AsRef<Path>>(doc: &Document, path: P) -> Result<(), IoError> {
        let path_ref = path.as_ref();
        let parent_dir = path_ref.parent().unwrap_or_else(|| Path::new("."));
        let temp_path = parent_dir.join(format!(".tmp_save_{}", uuid::Uuid::new_v4()));

        let file = File::create(&temp_path)?;
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        #[derive(serde::Serialize)]
        struct ProjectManifest<'a> {
            version: u32,
            title: &'a str,
            width: u32,
            height: u32,
            dpi: f32,
            layers: Vec<LayerMeta<'a>>,
            paths: Vec<crate::path::VectorPath>,
        }

        #[derive(serde::Serialize)]
        struct LayerMeta<'a> {
            id: String,
            name: &'a str,
            visible: bool,
            locked: bool,
            opacity: f32,
            blend_mode: crate::color::BlendMode,
            kind: crate::layer::LayerKind,
            clipping_mask: bool,
            mask_enabled: bool,
            style: Option<crate::style::LayerStyle>,
        }

        let manifest = ProjectManifest {
            version: 1,
            title: &doc.title,
            width: doc.width,
            height: doc.height,
            dpi: doc.dpi,
            paths: doc.paths.clone(),
            layers: doc.layers.iter().map(|l| LayerMeta {
                id: l.id.0.to_string(),
                name: &l.name,
                visible: l.visible,
                locked: l.locked,
                opacity: l.opacity,
                blend_mode: l.blend_mode,
                kind: l.kind.clone(),
                clipping_mask: l.clipping_mask,
                mask_enabled: l.mask_enabled,
                style: l.style.clone(),
            }).collect(),
        };

        zip.start_file("manifest.json", options)?;
        let manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
        zip.write_all(&manifest_bytes)?;

        for layer in &doc.layers {
            let entry_name = format!("layers/{}.bin", layer.id.0);
            zip.start_file(entry_name, options)?;
            zip.write_all(&layer.buffer.data)?;

            if let Some(mask) = &layer.mask {
                let mask_entry_name = format!("masks/{}.bin", layer.id.0);
                zip.start_file(mask_entry_name, options)?;
                zip.write_all(&mask.data)?;
            }
        }

        zip.finish()?;
        std::fs::rename(&temp_path, path_ref)?;
        Ok(())
    }

    pub fn load_project<P: AsRef<Path>>(path: P) -> Result<Document, IoError> {
        let file = File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        #[derive(serde::Deserialize)]
        struct ProjectManifest {
            #[allow(dead_code)]
            version: u32,
            title: String,
            width: u32,
            height: u32,
            dpi: f32,
            layers: Vec<LayerMeta>,
            #[serde(default)]
            paths: Vec<crate::path::VectorPath>,
        }

        #[derive(serde::Deserialize)]
        struct LayerMeta {
            id: String,
            name: String,
            visible: bool,
            locked: bool,
            opacity: f32,
            blend_mode: crate::color::BlendMode,
            #[serde(default = "default_layer_kind")]
            kind: crate::layer::LayerKind,
            #[serde(default)]
            clipping_mask: bool,
            #[serde(default)]
            mask_enabled: bool,
            #[serde(default)]
            style: Option<crate::style::LayerStyle>,
        }

        fn default_layer_kind() -> crate::layer::LayerKind {
            crate::layer::LayerKind::Raster
        }

        let manifest: ProjectManifest = {
            let mut manifest_file = archive.by_name("manifest.json")?;
            serde_json::from_reader(&mut manifest_file)?
        };

        let mut layers = Vec::new();

        for meta in manifest.layers {
            let data = {
                let entry_name = format!("layers/{}.bin", meta.id);
                let mut layer_file = archive.by_name(&entry_name)?;
                let mut data = Vec::new();
                layer_file.read_to_end(&mut data)?;
                data
            };

            let buffer = PixelBuffer::from_raw(manifest.width, manifest.height, data)
                .map_err(|e| IoError::InvalidFormat(e))?;
            let layer_id = uuid::Uuid::parse_str(&meta.id)
                .map(crate::layer::LayerId)
                .unwrap_or_else(|_| crate::layer::LayerId::new());

            let mask = {
                let mask_entry = format!("masks/{}.bin", meta.id);
                if let Ok(mut mask_file) = archive.by_name(&mask_entry) {
                    let mut mask_data = Vec::new();
                    if mask_file.read_to_end(&mut mask_data).is_ok() && mask_data.len() == (manifest.width * manifest.height) as usize {
                        Some(crate::selection::SelectionMask {
                            width: manifest.width,
                            height: manifest.height,
                            data: mask_data,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            layers.push(Layer {
                id: layer_id,
                name: meta.name,
                visible: meta.visible,
                locked: meta.locked,
                lock_alpha: false,
                opacity: meta.opacity,
                blend_mode: meta.blend_mode,
                buffer,
                kind: meta.kind,
                clipping_mask: meta.clipping_mask,
                mask,
                mask_enabled: meta.mask_enabled,
                style: meta.style,
            });
        }

        let active_layer_id = layers.last().map(|l| l.id);

        Ok(Document {
            id: crate::document::DocumentId::new(),
            title: manifest.title,
            width: manifest.width,
            height: manifest.height,
            dpi: manifest.dpi,
            layers,
            active_layer_id,
            selection: crate::selection::SelectionMask::new(manifest.width, manifest.height),
            paths: manifest.paths,
            history: crate::history::HistoryManager::new(50),
        })
    }
}
