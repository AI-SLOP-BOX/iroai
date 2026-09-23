use crate::document::Document;
use crate::layer::Layer;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PsdError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid PSD header signature or version")]
    InvalidHeader,
    #[error("Unsupported color mode or bit depth: {0}")]
    UnsupportedFormat(String),
}

pub struct PsdHandler;

impl PsdHandler {
    pub fn export_psd<P: AsRef<Path>>(doc: &Document, path: P) -> Result<(), PsdError> {
        let mut file = File::create(path)?;

        // 1. PSD Header (26 bytes)
        file.write_all(b"8BPS")?;
        file.write_all(&1u16.to_be_bytes())?;
        file.write_all(&[0u8; 6])?;
        file.write_all(&4u16.to_be_bytes())?;
        file.write_all(&(doc.height as u32).to_be_bytes())?;
        file.write_all(&(doc.width as u32).to_be_bytes())?;
        file.write_all(&8u16.to_be_bytes())?;
        file.write_all(&3u16.to_be_bytes())?;

        // 2. Color Mode Data
        file.write_all(&0u32.to_be_bytes())?;

        // 3. Image Resources
        file.write_all(&0u32.to_be_bytes())?;

        // 4. Layer and Mask Information Section
        let mut layer_info_bytes = Vec::new();
        let layer_count = doc.layers.len() as i16;
        layer_info_bytes.extend_from_slice(&layer_count.to_be_bytes());

        for layer in &doc.layers {
            layer_info_bytes.extend_from_slice(&0u32.to_be_bytes());
            layer_info_bytes.extend_from_slice(&0u32.to_be_bytes());
            layer_info_bytes.extend_from_slice(&(doc.height as u32).to_be_bytes());
            layer_info_bytes.extend_from_slice(&(doc.width as u32).to_be_bytes());
            layer_info_bytes.extend_from_slice(&4u16.to_be_bytes());

            let channel_data_len = (2 + (doc.width * doc.height)) as u32;
            layer_info_bytes.extend_from_slice(&0i16.to_be_bytes());
            layer_info_bytes.extend_from_slice(&channel_data_len.to_be_bytes());
            layer_info_bytes.extend_from_slice(&1i16.to_be_bytes());
            layer_info_bytes.extend_from_slice(&channel_data_len.to_be_bytes());
            layer_info_bytes.extend_from_slice(&2i16.to_be_bytes());
            layer_info_bytes.extend_from_slice(&channel_data_len.to_be_bytes());
            layer_info_bytes.extend_from_slice(&(-1i16).to_be_bytes());
            layer_info_bytes.extend_from_slice(&channel_data_len.to_be_bytes());

            layer_info_bytes.extend_from_slice(b"8BIM");
            let blend_key = match layer.blend_mode {
                crate::color::BlendMode::Normal => b"norm",
                crate::color::BlendMode::Multiply => b"mul ",
                crate::color::BlendMode::Screen => b"scrn",
                crate::color::BlendMode::Overlay => b"over",
                crate::color::BlendMode::Add => b"lddg",
                crate::color::BlendMode::Difference => b"diff",
                _ => b"norm",
            };
            layer_info_bytes.extend_from_slice(blend_key);

            let op_byte = (layer.opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
            layer_info_bytes.push(op_byte);
            layer_info_bytes.push(if layer.clipping_mask { 1 } else { 0 });

            let mut flags = 0u8;
            if !layer.visible {
                flags |= 0x02;
            }
            layer_info_bytes.push(flags);
            layer_info_bytes.push(0);

            let name_bytes = layer.name.as_bytes();
            let name_len = name_bytes.len().min(255) as u8;
            let padded_name_len = (name_len + 1 + 3) & !3;

            let extra_len = 4 + 4 + (padded_name_len as usize);
            layer_info_bytes.extend_from_slice(&(extra_len as u32).to_be_bytes());
            layer_info_bytes.extend_from_slice(&0u32.to_be_bytes());
            layer_info_bytes.extend_from_slice(&0u32.to_be_bytes());

            layer_info_bytes.push(name_len);
            layer_info_bytes.extend_from_slice(&name_bytes[..name_len as usize]);
            let pad_count = padded_name_len - (name_len + 1);
            for _ in 0..pad_count {
                layer_info_bytes.push(0);
            }
        }

        for layer in &doc.layers {
            let total_pixels = (doc.width * doc.height) as usize;
            for c_idx in 0..4 {
                layer_info_bytes.extend_from_slice(&0u16.to_be_bytes());
                let mut channel_bytes = Vec::with_capacity(total_pixels);
                for i in 0..total_pixels {
                    let byte_offset = i * 4;
                    let val = match c_idx {
                        0 => layer.buffer.data[byte_offset],
                        1 => layer.buffer.data[byte_offset + 1],
                        2 => layer.buffer.data[byte_offset + 2],
                        3 => layer.buffer.data[byte_offset + 3],
                        _ => 0,
                    };
                    channel_bytes.push(val);
                }
                layer_info_bytes.extend_from_slice(&channel_bytes);
            }
        }

        let layer_and_mask_len = (4 + layer_info_bytes.len()) as u32;
        file.write_all(&layer_and_mask_len.to_be_bytes())?;
        file.write_all(&(layer_info_bytes.len() as u32).to_be_bytes())?;
        file.write_all(&layer_info_bytes)?;

        // 5. Composite Image Data
        let composite = doc.composite();
        file.write_all(&0u16.to_be_bytes())?;
        let total_pixels = (doc.width * doc.height) as usize;
        for c in 0..4 {
            for i in 0..total_pixels {
                let px = composite.data[i * 4 + c];
                file.write_all(&[px])?;
            }
        }

        Ok(())
    }

    pub fn load_psd<P: AsRef<Path>>(path: P) -> Result<Document, PsdError> {
        let mut file = File::open(path)?;
        let mut header = [0u8; 26];
        file.read_exact(&mut header)?;

        if &header[0..4] != b"8BPS" {
            return Err(PsdError::InvalidHeader);
        }

        let version = u16::from_be_bytes([header[4], header[5]]);
        if version != 1 {
            return Err(PsdError::InvalidHeader);
        }

        let channels = u16::from_be_bytes([header[12], header[13]]);
        let height = u32::from_be_bytes([header[14], header[15], header[16], header[17]]);
        let width = u32::from_be_bytes([header[18], header[19], header[20], header[21]]);
        let depth = u16::from_be_bytes([header[22], header[23]]);
        let color_mode = u16::from_be_bytes([header[24], header[25]]);

        if depth != 8 || color_mode != 3 {
            return Err(PsdError::UnsupportedFormat(format!("Only 8-bit RGB PSD is supported (got depth={}, mode={})", depth, color_mode)));
        }

        let mut buf4 = [0u8; 4];
        file.read_exact(&mut buf4)?;
        let color_mode_len = u32::from_be_bytes(buf4);
        std::io::copy(&mut std::io::Read::by_ref(&mut file).take(color_mode_len as u64), &mut std::io::sink())?;

        file.read_exact(&mut buf4)?;
        let img_res_len = u32::from_be_bytes(buf4);
        std::io::copy(&mut std::io::Read::by_ref(&mut file).take(img_res_len as u64), &mut std::io::sink())?;

        file.read_exact(&mut buf4)?;
        let layer_section_len = u32::from_be_bytes(buf4);

        if layer_section_len > 0 {
            file.read_exact(&mut buf4)?;
            let layer_info_len = u32::from_be_bytes(buf4);
            if layer_info_len > 0 {
                let mut buf2 = [0u8; 2];
                file.read_exact(&mut buf2)?;
                let layer_count = i16::from_be_bytes(buf2).abs() as usize;

                let mut parsed_layers = Vec::new();

                for _ in 0..layer_count {
                    let mut rect_bytes = [0u8; 16];
                    file.read_exact(&mut rect_bytes)?;

                    file.read_exact(&mut buf2)?;
                    let num_channels = u16::from_be_bytes(buf2);

                    for _ in 0..num_channels {
                        let mut ch_info = [0u8; 6];
                        file.read_exact(&mut ch_info)?;
                    }

                    let mut blend_sig = [0u8; 4];
                    file.read_exact(&mut blend_sig)?;
                    let mut blend_key = [0u8; 4];
                    file.read_exact(&mut blend_key)?;

                    let mut op_byte = [0u8; 1];
                    file.read_exact(&mut op_byte)?;
                    let opacity = op_byte[0] as f32 / 255.0;

                    let mut clip_byte = [0u8; 1];
                    file.read_exact(&mut clip_byte)?;
                    let clipping_mask = clip_byte[0] > 0;

                    let mut flags_byte = [0u8; 1];
                    file.read_exact(&mut flags_byte)?;
                    let visible = (flags_byte[0] & 0x02) == 0;

                    let mut filler = [0u8; 1];
                    file.read_exact(&mut filler)?;

                    file.read_exact(&mut buf4)?;
                    let extra_len = u32::from_be_bytes(buf4);

                    let mut extra_data = vec![0u8; extra_len as usize];
                    file.read_exact(&mut extra_data)?;

                    let name = if extra_len >= 8 {
                        let name_len = extra_data[8] as usize;
                        if extra_data.len() >= 9 + name_len {
                            String::from_utf8_lossy(&extra_data[9..9 + name_len]).to_string()
                        } else {
                            "PSD Layer".to_string()
                        }
                    } else {
                        "PSD Layer".to_string()
                    };

                    let blend_mode = match &blend_key {
                        b"mul " => crate::color::BlendMode::Multiply,
                        b"scrn" => crate::color::BlendMode::Screen,
                        b"over" => crate::color::BlendMode::Overlay,
                        b"lddg" => crate::color::BlendMode::Add,
                        b"diff" => crate::color::BlendMode::Difference,
                        _ => crate::color::BlendMode::Normal,
                    };

                    let mut layer = Layer::new_empty(width, height, name);
                    layer.opacity = opacity;
                    layer.visible = visible;
                    layer.blend_mode = blend_mode;
                    layer.clipping_mask = clipping_mask;
                    parsed_layers.push(layer);
                }

                for layer in &mut parsed_layers {
                    let total_pixels = (width * height) as usize;
                    for c_idx in 0..4 {
                        let mut comp_byte = [0u8; 2];
                        if file.read_exact(&mut comp_byte).is_ok() {
                            let compression = u16::from_be_bytes(comp_byte);
                            let ch_data = if compression == 1 {
                                // RLE PackBits compression (Photoshop standard)
                                let row_count = height as usize;
                                let mut byte_counts = Vec::with_capacity(row_count);
                                for _ in 0..row_count {
                                    let mut row_len_bytes = [0u8; 2];
                                    if file.read_exact(&mut row_len_bytes).is_ok() {
                                        byte_counts.push(u16::from_be_bytes(row_len_bytes) as usize);
                                    }
                                }
                                let mut decoded_channel = Vec::with_capacity(total_pixels);
                                for row_len in byte_counts {
                                    let mut compressed_row = vec![0u8; row_len];
                                    let _ = file.read_exact(&mut compressed_row);
                                    if let Ok(decoded_row) = crate::packbits::PackBits::decode(&compressed_row, width as usize) {
                                        decoded_channel.extend_from_slice(&decoded_row);
                                    } else {
                                        decoded_channel.resize(decoded_channel.len() + width as usize, 0);
                                    }
                                }
                                decoded_channel
                            } else {
                                // Raw uncompressed
                                let mut raw_data = vec![0u8; total_pixels];
                                let _ = file.read_exact(&mut raw_data);
                                raw_data
                            };

                            for i in 0..total_pixels.min(ch_data.len()) {
                                let offset = i * 4;
                                match c_idx {
                                    0 => layer.buffer.data[offset] = ch_data[i],
                                    1 => layer.buffer.data[offset + 1] = ch_data[i],
                                    2 => layer.buffer.data[offset + 2] = ch_data[i],
                                    3 => layer.buffer.data[offset + 3] = ch_data[i],
                                    _ => {}
                                }
                            }
                        }
                    }
                }

                if !parsed_layers.is_empty() {
                    let mut doc = Document::new(width, height, "PSD Import");
                    doc.layers = parsed_layers;
                    doc.active_layer_id = doc.layers.last().map(|l| l.id);
                    return Ok(doc);
                }
            }
        }

        let mut comp_byte = [0u8; 2];
        file.read_exact(&mut comp_byte)?;
        let total_pixels = (width * height) as usize;
        let mut composite_buf = crate::buffer::PixelBuffer::new(width, height);
        for c in 0..channels.min(4) {
            let mut ch_data = vec![0u8; total_pixels];
            let _ = file.read_exact(&mut ch_data);
            for i in 0..total_pixels {
                composite_buf.data[i * 4 + c as usize] = ch_data[i];
            }
        }

        let layer = Layer::new_from_buffer("PSD Background", composite_buf);
        let doc = Document::new_with_layer(layer, "Imported PSD");
        Ok(doc)
    }
}
