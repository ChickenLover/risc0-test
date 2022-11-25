// The BmpHeader always has a size of 14 bytes
const BMP_HEADER_SIZE: u64 = 14;

use core::convert::TryInto;

use alloc::{string::String};

// Import structs/functions defined in lib.rs
use super::*;
use self::BmpErrorKind::*;

/// A result type, either containing an `Image` or a `BmpError`.
pub type BmpResult<T> = Result<T, BmpError>;

pub fn u32_from_slice(slice: &[u8]) -> u32 {
    u32::from_ne_bytes(slice.split_at(4).0.try_into().unwrap())
}

pub fn u16_from_slice(slice: &[u8]) -> u16 {
    u16::from_ne_bytes(slice.split_at(2).0.try_into().unwrap())
}

/// The error type returned if the decoding of an image from disk fails.
#[derive(Debug)]
pub struct BmpError {
    pub kind: BmpErrorKind,
    pub details: String,
}

impl BmpError {
    fn new<T: AsRef<str>>(kind: BmpErrorKind, details: T) -> BmpError {
        BmpError {
            kind: kind,
            details: String::from(details.as_ref()),
        }
    }
}

/// The different kinds of possible BMP errors.
#[derive(Debug)]
pub enum BmpErrorKind {
    WrongMagicNumbers,
    UnsupportedBitsPerPixel,
    UnsupportedCompressionType,
    UnsupportedBmpVersion,
    UnsupportedHeader,
}

impl AsRef<str> for BmpErrorKind {
    fn as_ref(&self) -> &str {
        match *self {
            WrongMagicNumbers => "Wrong magic numbers",
            UnsupportedBitsPerPixel => "Unsupported bits per pixel",
            UnsupportedCompressionType => "Unsupported compression type",
            UnsupportedBmpVersion => "Unsupported BMP version",
            _ => "BMP Error",
        }
    }
}

pub fn decode_image(bmp_data: &[u8]) -> BmpResult<Image> {
    read_bmp_id(bmp_data)?;
    let header = read_bmp_header(bmp_data)?;
    let dib_header = read_bmp_dib_header(bmp_data)?;

    let color_palette = read_color_palette(bmp_data, &dib_header)?;

    let width = dib_header.width.abs() as u32;
    let height = dib_header.height.abs() as u32;
    let padding = width % 4;

    let data = match color_palette {
        Some(ref palette) => {
            read_indexes(
                bmp_data,
                &palette,
                width as usize,
                height as usize,
                dib_header.bits_per_pixel,
                header.pixel_offset as usize,
            )?
        }
        None => read_pixels(bmp_data, width, height, header.pixel_offset)?,
    };

    let image = Image {
        header,
        dib_header: BmpDibHeader::new(width as i32, height as i32),
        color_palette,
        width,
        height,
        padding,
        data,
    };

    Ok(image)
}

fn read_bmp_id(bmp_data: &[u8]) -> BmpResult<()> {
    let mut bm = [0, 0];
    bm.clone_from_slice(&bmp_data[..2]);

    if bm == b"BM"[..] {
        Ok(())
    } else {
        Err(BmpError::new(
            WrongMagicNumbers,
            "Expected [66, 77], but was {:?}",
        ))
    }
}

fn read_bmp_header(bmp_data: &[u8]) -> BmpResult<BmpHeader> {
    let header = BmpHeader {
        file_size: u32_from_slice(&bmp_data[2..6]),
        creator1: u16_from_slice(&bmp_data[6..8]),
        creator2: u16_from_slice(&bmp_data[8..10]),
        pixel_offset: u32_from_slice(&bmp_data[10..14]),
    };

    Ok(header)
}

fn read_bmp_dib_header(bmp_data: &[u8]) -> BmpResult<BmpDibHeader> {
    let dib_header = BmpDibHeader {
        header_size: u32_from_slice(&bmp_data[14..18]),
        width: u32_from_slice(&bmp_data[18..22]) as i32,
        height: u32_from_slice(&bmp_data[22..26]) as i32,
        num_planes: u16_from_slice(&bmp_data[26..28]),
        bits_per_pixel: u16_from_slice(&bmp_data[28..30]),
        compress_type: u32_from_slice(&bmp_data[30..34]),
        data_size: u32_from_slice(&bmp_data[34..38]),
        hres: u32_from_slice(&bmp_data[38..42]) as i32,
        vres: u32_from_slice(&bmp_data[42..46]) as i32,
        num_colors: u32_from_slice(&bmp_data[46..50]),
        num_imp_colors: u32_from_slice(&bmp_data[50..54]),
    };

    match BmpVersion::from_dib_header(&dib_header) {
        // V3 is the only version that is "fully" supported (decompressed images are the exception)
        // We will also attempt to decode v4 and v5, but we ignore all the additional data in the header.
        // This should not impose a big problem because neither decompression, nor 16 and 32-bit images are supported,
        // so the decoding will likely fail due to these constraints either way.
        Some(BmpVersion::Three) |
        Some(BmpVersion::Four) |
        Some(BmpVersion::Five) => (),
        // Otherwise, report the errors
        Some(other) => return Err(BmpError::new(UnsupportedBmpVersion, other)),
        None => {
            return Err(BmpError::new(
                UnsupportedHeader,
                "Only simple BMP images of version 3, 4, and 5 are currently supported. \
                Connot decode the image for the following header: {:?}",
                ),
            );
        }
    }

    match dib_header.bits_per_pixel {
        // Currently supported
        1 | 4 | 8 | 24 => (),
        _other => {
            return Err(BmpError::new(
                UnsupportedBitsPerPixel,
                "Only 1, 4, 8, and 24 bits per pixel are currently supported, was: {}",
            ))
        }
    }

    match CompressionType::from_u32(dib_header.compress_type) {
        CompressionType::Uncompressed => (),
        other => return Err(BmpError::new(UnsupportedCompressionType, other)),
    }

    Ok(dib_header)
}

fn read_color_palette(
    bmp_data: &[u8],
    dh: &BmpDibHeader,
) -> BmpResult<Option<Vec<Pixel>>> {
    let num_entries = match dh.bits_per_pixel {
        // We have a color_palette if the num_colors in the dib header is not zero
        _ if dh.num_colors != 0 => dh.num_colors as usize,
        // Or if there are 8 or less bits per pixel
        bpp @ 1 | bpp @ 4 | bpp @ 8 => 1 << bpp,
        _ => return Ok(None),
    };

    let num_bytes = match BmpVersion::from_dib_header(&dh) {
        // Three bytes for v2. Though, this is currently not supported
        Some(BmpVersion::Two) => return Err(BmpError::new(UnsupportedBmpVersion, BmpVersion::Two)),
        // Each entry in the color_palette is four bytes for v3, v4, and v5
        _ => 4,
    };


    let offset = (BMP_HEADER_SIZE + dh.header_size as u64) as usize;
    let px = &mut [0; 4][0..num_bytes as usize];
    let mut color_palette = Vec::with_capacity(num_entries);
    for i in 0..num_entries {
        px.copy_from_slice(&bmp_data[offset + i * 4 .. offset + (i + 1) * 4]);
        color_palette.push(px!(px[2], px[1], px[0]));
    }

    Ok(Some(color_palette))
}

fn read_indexes(
    bmp_data: &[u8],
    palette: &Vec<Pixel>,
    width: usize,
    height: usize,
    bpp: u16,
    offset: usize,
) -> BmpResult<Vec<Pixel>> {
    let mut data = Vec::with_capacity(height * width);
    // Number of bytes to read from each row, varies based on bits_per_pixel
    let bytes_per_row = (width as f64 / (8.0 / bpp as f64)) as usize;
    for y in 0..height {
        let padding = match bytes_per_row % 4 {
            0 => 0,
            other => 4 - other,
        };
        let start = offset + (bytes_per_row + padding) * y;
        let bytes = &bmp_data[start..start + bytes_per_row];

        for i in bit_index(&bytes, bpp as usize, width as usize) {
            data.push(palette[i]);
        }
    }
    Ok(data)
}

fn read_pixels(
    bmp_data: &[u8],
    width: u32,
    height: u32,
    offset: u32,
) -> BmpResult<Vec<Pixel>> {
    let mut data = Vec::with_capacity((height * width) as usize);
    // read pixels until padding
    let mut px = [0; 3];
    for y in 0..height {
        for x in 0..width {
            let lr = (y * width + x) as usize;
            px.copy_from_slice(&bmp_data[offset as usize + lr * 4 .. offset as usize + (lr + 1) * 4 - 1]);
            data.push(px!(px[2], px[1], px[0]));
        }
    }
    Ok(data)
}

const BITS: usize = 8;

#[derive(Debug)]
struct BitIndex<'a> {
    size: usize,
    nbits: usize,
    bits_left: usize,
    mask: u8,
    bytes: &'a [u8],
    index: usize,
}

fn bit_index<'a>(bytes: &'a [u8], nbits: usize, size: usize) -> BitIndex {
    let bits_left = BITS - nbits;
    BitIndex {
        size,
        nbits,
        bits_left,
        mask: (!0 as u8 >> bits_left),
        bytes,
        index: 0,
    }
}

impl<'a> Iterator for BitIndex<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        let n = self.index / BITS;
        let offset = self.bits_left - self.index % BITS;

        self.index += self.nbits;

        if self.size == 0 {
            None
        } else {
            self.size -= 1;
            self.bytes.get(n).map(|&block| {
                ((block & self.mask << offset) >> offset) as usize
            })
        }
    }
}