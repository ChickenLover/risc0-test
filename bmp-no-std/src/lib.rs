#![deny(warnings)]
#![cfg_attr(test, deny(warnings))]
#![no_std]
// Expose decoder's public types, structs, and enums
pub use decoder::{BmpError, BmpErrorKind, BmpResult};

/// Macro to generate a `Pixel` from `r`, `g` and `b` values.
#[macro_export]
macro_rules! px {
    ($r:expr, $g:expr, $b:expr) => {
        Pixel { r: $r as u8, g: $g as u8, b: $b as u8 }
    }
}

macro_rules! file_size {
    ($bpp:expr, $width:expr, $height:expr) => {{
        let header_size = 2 + 12 + 40;
        // find row size in bytes, round up to 4 bytes (padding)
        let row_size = (($bpp as f32 * $width as f32 + 31.0) / 32.0) as u32 * 4;
        (header_size as u32, $height as u32 * row_size)
    }}
}

/// Common color constants accessible by names.
pub mod consts;

mod decoder;

extern crate alloc;
use alloc::vec::Vec;

/// The pixel data used in the `Image`.
///
/// It has three values for the `red`, `blue` and `green` color channels, respectively.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Pixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Pixel {
    /// Creates a new `Pixel`.
    pub fn new(r: u8, g: u8, b: u8) -> Pixel {
        Pixel { r: r, g: g, b: b }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum BmpVersion {
    Two,
    Three,
    ThreeNT,
    Four,
    Five,
}

impl BmpVersion {
    fn from_dib_header(dib_header: &BmpDibHeader) -> Option<BmpVersion> {
        match dib_header.header_size {
            12 => Some(BmpVersion::Two),
            40 if dib_header.compress_type == 3 => Some(BmpVersion::ThreeNT),
            40 => Some(BmpVersion::Three),
            108 => Some(BmpVersion::Four),
            124 => Some(BmpVersion::Five),
            _ => None,
        }
    }
}

impl AsRef<str> for BmpVersion {
    fn as_ref(&self) -> &str {
        match *self {
            BmpVersion::Two => "BMP Version 2",
            BmpVersion::Three => "BMP Version 3",
            BmpVersion::ThreeNT => "BMP Version 3 NT",
            BmpVersion::Four => "BMP Version 4",
            BmpVersion::Five => "BMP Version 5",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum CompressionType {
    Uncompressed,
    Rle8bit,
    Rle4bit,
    // Only for BMP version 4
    BitfieldsEncoding,
}

impl CompressionType {
    fn from_u32(val: u32) -> CompressionType {
        match val {
            1 => CompressionType::Rle8bit,
            2 => CompressionType::Rle4bit,
            3 => CompressionType::BitfieldsEncoding,
            _ => CompressionType::Uncompressed,
        }
    }
}

impl AsRef<str> for CompressionType {
    fn as_ref(&self) -> &str {
        match *self {
            CompressionType::Rle8bit => "RLE 8-bit",
            CompressionType::Rle4bit => "RLE 4-bit",
            CompressionType::BitfieldsEncoding => "Bitfields Encoding",
            CompressionType::Uncompressed => "Uncompressed",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BmpHeader {
    file_size: u32,
    creator1: u16,
    creator2: u16,
    pixel_offset: u32,
}

impl BmpHeader {
    fn new(header_size: u32, data_size: u32) -> BmpHeader {
        BmpHeader {
            file_size: header_size + data_size,
            creator1: 0, /* Unused */
            creator2: 0, /* Unused */
            pixel_offset: header_size,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BmpDibHeader {
    header_size: u32,
    width: i32,
    height: i32,
    num_planes: u16,
    bits_per_pixel: u16,
    compress_type: u32,
    data_size: u32,
    hres: i32,
    vres: i32,
    num_colors: u32,
    num_imp_colors: u32,
}

impl BmpDibHeader {
    fn new(width: i32, height: i32) -> BmpDibHeader {
        let (_, pixel_array_size) = file_size!(24, width, height);
        BmpDibHeader {
            header_size: 40,
            width: width,
            height: height,
            num_planes: 1,
            bits_per_pixel: 24,
            compress_type: 0,
            data_size: pixel_array_size,
            hres: 1000,
            vres: 1000,
            num_colors: 0,
            num_imp_colors: 0,
        }
    }
}

/// The image type provided by the library.
///
/// It exposes functions to initialize or read BMP images from disk, common modification of pixel
/// data, and saving to disk.
///
/// The image is accessed in row-major order from top to bottom,
/// where point (0, 0) is defined to be in the upper left corner of the image.
///
/// Currently, only uncompressed BMP images are supported.
#[derive(Clone, Eq, PartialEq)]
pub struct Image {
    header: BmpHeader,
    dib_header: BmpDibHeader,
    color_palette: Option<Vec<Pixel>>,
    width: u32,
    height: u32,
    padding: u32,
    data: Vec<Pixel>,
}

impl Image {
    /// Returns a new BMP Image with the `width` and `height` specified. It is initialized to
    /// a black image by default.
    ///
    /// # Example
    ///
    /// ```
    /// let mut img = bmp::Image::new(100, 80);
    /// ```
    pub fn new(width: u32, height: u32) -> Image {
        let mut data = Vec::with_capacity((width * height) as usize);
        for _ in 0..width * height {
            data.push(px!(0, 0, 0));
        }

        let (header_size, data_size) = file_size!(24, width, height);
        Image {
            header: BmpHeader::new(header_size, data_size),
            dib_header: BmpDibHeader::new(width as i32, height as i32),
            color_palette: None,
            width: width,
            height: height,
            padding: width % 4,
            data: data,
        }
    }

    /// Returns the `width` of the Image.
    #[inline]
    pub fn get_width(&self) -> u32 {
        self.width
    }

    /// Returns the `height` of the Image.
    #[inline]
    pub fn get_height(&self) -> u32 {
        self.height
    }

    /// Set the pixel value at the position of `width` and `height`.
    ///
    /// # Example
    ///
    /// ```
    /// let mut img = bmp::Image::new(100, 80);
    /// img.set_pixel(10, 10, bmp::consts::RED);
    /// ```
    #[inline]
    pub fn set_pixel(&mut self, x: u32, y: u32, val: Pixel) {
        self.data[((self.height - y - 1) * self.width + x) as usize] = val;
    }

    /// Returns the pixel value at the position of `width` and `height`.
    ///
    /// # Example
    ///
    /// ```
    /// let img = bmp::Image::new(100, 80);
    /// assert_eq!(bmp::consts::BLACK, img.get_pixel(10, 10));
    /// ```
    #[inline]
    pub fn get_pixel(&self, x: u32, y: u32) -> Pixel {
        self.data[((self.height - y - 1) * self.width + x) as usize]
    }

    /// Returns a new `ImageIndex` that iterates over the image dimensions in top-bottom order.
    ///
    /// # Example
    ///
    /// ```
    /// let mut img = bmp::Image::new(100, 100);
    /// for (x, y) in img.coordinates() {
    ///     img.set_pixel(x, y, bmp::consts::BLUE);
    /// }
    /// ```
    #[inline]
    pub fn coordinates(&self) -> ImageIndex {
        ImageIndex::new(self.width as u32, self.height as u32)
    }

}

/// An `Iterator` returning the `x` and `y` coordinates of an image.
///
/// It supports iteration over an image in row-major order,
/// starting from in the upper left corner of the image.
#[derive(Clone, Copy)]
pub struct ImageIndex {
    width: u32,
    height: u32,
    x: u32,
    y: u32,
}

impl ImageIndex {
    fn new(width: u32, height: u32) -> ImageIndex {
        ImageIndex {
            width,
            height,
            x: 0,
            y: 0,
        }
    }
}

impl Iterator for ImageIndex {
    type Item = (u32, u32);

    fn next(&mut self) -> Option<(u32, u32)> {
        if self.x < self.width && self.y < self.height {
            let this = Some((self.x, self.y));
            self.x += 1;
            if self.x == self.width {
                self.x = 0;
                self.y += 1;
            }
            this
        } else {
            None
        }
    }
}

/// Attempts to construct a new `Image` from the given reader.
/// Returns a `BmpResult`, either containing an `Image` or a `BmpError`.
pub fn from_bytes(bytes: &[u8]) -> BmpResult<Image> {
    decoder::decode_image(bytes)
}