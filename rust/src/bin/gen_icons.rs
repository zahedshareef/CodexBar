//! Generate icon files for Tauri.
//! This creates a simple icon.ico file for Windows.

use std::fs::File;
use std::io::Write;

fn main() {
    std::fs::create_dir_all("icons").expect("Failed to create icons dir");

    // Create ICO file
    create_ico("icons/icon.ico");
    println!("Generated icons/icon.ico");

    // Create PNG file
    create_png("icons/icon.png", 256);
    println!("Generated icons/icon.png");
}

fn create_ico(path: &str) {
    let mut file = File::create(path).expect("Failed to create ICO file");

    // Generate 32x32 icon data
    let size = 32u32;
    let rgba = generate_icon_rgba(size);

    // ICO header
    let reserved: u16 = 0;
    let image_type: u16 = 1; // ICO
    let image_count: u16 = 1;

    file.write_all(&reserved.to_le_bytes()).unwrap();
    file.write_all(&image_type.to_le_bytes()).unwrap();
    file.write_all(&image_count.to_le_bytes()).unwrap();

    // ICO directory entry
    let width: u8 = size as u8;
    let height: u8 = size as u8;
    let palette: u8 = 0;
    let reserved2: u8 = 0;
    let color_planes: u16 = 1;
    let bits_per_pixel: u16 = 32;

    // BMP size calculation
    let bmp_header_size = 40u32;
    let pixel_data_size = size * size * 4;
    let mask_size = size.div_ceil(32) * 4 * size; // AND mask
    let bmp_size = bmp_header_size + pixel_data_size + mask_size;

    let data_offset: u32 = 6 + 16; // ICO header + directory entry

    file.write_all(&[width]).unwrap();
    file.write_all(&[height]).unwrap();
    file.write_all(&[palette]).unwrap();
    file.write_all(&[reserved2]).unwrap();
    file.write_all(&color_planes.to_le_bytes()).unwrap();
    file.write_all(&bits_per_pixel.to_le_bytes()).unwrap();
    file.write_all(&bmp_size.to_le_bytes()).unwrap();
    file.write_all(&data_offset.to_le_bytes()).unwrap();

    // BMP info header (BITMAPINFOHEADER)
    file.write_all(&bmp_header_size.to_le_bytes()).unwrap(); // biSize
    file.write_all(&(size as i32).to_le_bytes()).unwrap(); // biWidth
    file.write_all(&((size * 2) as i32).to_le_bytes()).unwrap(); // biHeight (doubled for ICO)
    file.write_all(&1u16.to_le_bytes()).unwrap(); // biPlanes
    file.write_all(&32u16.to_le_bytes()).unwrap(); // biBitCount
    file.write_all(&0u32.to_le_bytes()).unwrap(); // biCompression
    file.write_all(&(pixel_data_size + mask_size).to_le_bytes())
        .unwrap(); // biSizeImage
    file.write_all(&0i32.to_le_bytes()).unwrap(); // biXPelsPerMeter
    file.write_all(&0i32.to_le_bytes()).unwrap(); // biYPelsPerMeter
    file.write_all(&0u32.to_le_bytes()).unwrap(); // biClrUsed
    file.write_all(&0u32.to_le_bytes()).unwrap(); // biClrImportant

    // Pixel data (BGRA, bottom-up)
    for y in (0..size).rev() {
        for x in 0..size {
            let idx = ((y * size + x) * 4) as usize;
            let r = rgba[idx];
            let g = rgba[idx + 1];
            let b = rgba[idx + 2];
            let a = rgba[idx + 3];
            file.write_all(&[b, g, r, a]).unwrap();
        }
    }

    // AND mask (all zeros = fully opaque)
    let mask_row_size = size.div_ceil(32) * 4;
    for _ in 0..size {
        for _ in 0..mask_row_size {
            file.write_all(&[0u8]).unwrap();
        }
    }
}

fn create_png(path: &str, size: u32) {
    let mut file = File::create(path).expect("Failed to create PNG file");
    let rgba = generate_icon_rgba(size);

    // PNG signature
    file.write_all(&[137, 80, 78, 71, 13, 10, 26, 10]).unwrap();

    // IHDR chunk
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&size.to_be_bytes());
    ihdr.extend_from_slice(&size.to_be_bytes());
    ihdr.push(8); // bit depth
    ihdr.push(6); // color type (RGBA)
    ihdr.push(0); // compression
    ihdr.push(0); // filter
    ihdr.push(0); // interlace

    write_png_chunk(&mut file, b"IHDR", &ihdr);

    // IDAT chunk
    let mut raw_data = Vec::new();
    for y in 0..size {
        raw_data.push(0); // filter byte (none)
        let row_start = (y * size * 4) as usize;
        let row_end = row_start + (size * 4) as usize;
        raw_data.extend_from_slice(&rgba[row_start..row_end]);
    }

    let compressed = compress_zlib(&raw_data);
    write_png_chunk(&mut file, b"IDAT", &compressed);

    // IEND chunk
    write_png_chunk(&mut file, b"IEND", &[]);
}

fn generate_icon_rgba(size: u32) -> Vec<u8> {
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);

    // Green color scheme
    let (r, g, b) = (76u8, 175u8, 80u8);
    let letter_color = (255u8, 255u8, 255u8, 255u8);
    let bg_color = (r, g, b, 255u8);
    let border_color = (r / 2, g / 2, b / 2, 255u8);

    let scale = size as f32 / 32.0;
    let margin = (2.0 * scale) as u32;
    let border = (1.0 * scale).max(1.0) as u32;

    let cx = size / 2;
    let cy = size / 2;
    let outer_radius = (10.0 * scale) as u32;
    let inner_radius = (5.0 * scale) as u32;

    for y in 0..size {
        for x in 0..size {
            let in_bounds = x >= margin && x < size - margin && y >= margin && y < size - margin;
            let in_border = in_bounds
                && (x < margin + border
                    || x >= size - margin - border
                    || y < margin + border
                    || y >= size - margin - border);

            let is_c = is_letter_c(x, y, cx, cy, outer_radius, inner_radius);

            let pixel = if !in_bounds {
                (0u8, 0u8, 0u8, 0u8)
            } else if in_border {
                border_color
            } else if is_c {
                letter_color
            } else {
                bg_color
            };

            rgba.extend_from_slice(&[pixel.0, pixel.1, pixel.2, pixel.3]);
        }
    }

    rgba
}

fn is_letter_c(x: u32, y: u32, cx: u32, cy: u32, outer_r: u32, inner_r: u32) -> bool {
    let dx = (x as i32 - cx as i32).unsigned_abs();
    let dy = (y as i32 - cy as i32).unsigned_abs();
    let dist_sq = dx * dx + dy * dy;

    let in_ring = dist_sq >= inner_r * inner_r && dist_sq <= outer_r * outer_r;
    let gap_threshold = (2.0 * (cx as f32 / 16.0)) as u32;
    let is_gap = x > cx && dy < dx / 2 + gap_threshold;

    in_ring && !is_gap
}

fn write_png_chunk(file: &mut File, chunk_type: &[u8; 4], data: &[u8]) {
    let len = (data.len() as u32).to_be_bytes();
    file.write_all(&len).unwrap();
    file.write_all(chunk_type).unwrap();
    file.write_all(data).unwrap();

    let mut crc_data = Vec::new();
    crc_data.extend_from_slice(chunk_type);
    crc_data.extend_from_slice(data);
    let crc = crc32(&crc_data);
    file.write_all(&crc.to_be_bytes()).unwrap();
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xffffffff;
    for byte in data {
        crc ^= *byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xedb88320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

fn compress_zlib(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();

    // Zlib header
    result.push(0x78);
    result.push(0x01);

    // Deflate blocks
    let mut offset = 0;
    while offset < data.len() {
        let remaining = data.len() - offset;
        let block_size = remaining.min(65535);
        let is_final = offset + block_size >= data.len();

        result.push(if is_final { 0x01 } else { 0x00 });
        let len = block_size as u16;
        let nlen = !len;
        result.extend_from_slice(&len.to_le_bytes());
        result.extend_from_slice(&nlen.to_le_bytes());
        result.extend_from_slice(&data[offset..offset + block_size]);

        offset += block_size;
    }

    // Adler-32
    let adler = adler32(data);
    result.extend_from_slice(&adler.to_be_bytes());

    result
}

fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for byte in data {
        a = (a + *byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}
