//! `bmper` bmp image metadata parser and more...
//! ## Device-independent bitmaps
//!
//! The format for a DIB is the following (for more information, see Bitmap Storage ):
//!
//! *  a BITMAPFILEHEADER structure
//! *  either a BITMAPINFOHEADER, a BITMAPV4HEADER, or a BITMAPV5HEADER structure.
//! *  an optional color table, which is a set of RGBQUAD structures
//! *  the bitmap data
//! *  optional Profile data
//!
//! A color table describes how pixel values correspond to RGB color values.
//! RGB is a model for describing colors that are produced by emitting light.
//!
//! The four types of bitmap headers are differentiated by the Size member,
//! which is the first DWORD in each of the structures.
//!
//! ## For more info
//! see https://msdn.microsoft.com/en-us/library/dd183386(v=vs.85).aspx
//! and https://msdn.microsoft.com/en-us/library/dd183391(v=vs.85).aspx

use std::{self, fmt};
use std::error::Error;
use std::fs::File;
use std::path::Path;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use rand::{self, Rng};

use std::process;

use encoding::Rle8;

#[derive(Debug, Copy, Clone)]
pub enum BMPCompression {
    /// An uncompressed format.
    RGB,
    /// A run-length encoded (RLE) format for bitmaps with 8 bpp.
    /// The compression format is a 2-byte format consisting of a count byte
    /// followed by a byte containing a color index.
    RLE8,
    /// An RLE format for bitmaps with 4 bpp.
    /// The compression format is a 2-byte format consisting of a count byte
    /// followed by two word-length color indexes.
    RLE4,
    /// Specifies that the bitmap is not compressed and that the color table
    /// consists of three DWORD color masks that specify
    /// the red, green, and blue components, respectively, of each pixel.
    /// This is valid when used with 16- and 32-bpp bitmaps.
    BITFIELDS,
    /// Indicates that the image is a JPEG image.
    JPEG,
    ///  Indicates that the image is a PNG image.
    PNG,
}

impl fmt::Display for BMPCompression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match *self {
            BMPCompression::RGB => "Uncompressed",
            BMPCompression::RLE8 => "Run-Length Encoded (RLE) with 8 bpp",
            BMPCompression::RLE4 => "Run-Length Encoded (RLE) with 4 bpp",
            BMPCompression::BITFIELDS => "Uncompressed bitfields",
            BMPCompression::JPEG => "Bitmap is JPEG image",
            BMPCompression::PNG => "Bitmap is PNG image",
        })
    }
}

impl BMPCompression {
    pub fn from_bytes(b: i32) -> io::Result<BMPCompression> {
        match b {
            0 => Ok(BMPCompression::RGB),
            1 => Ok(BMPCompression::RLE8),
            2 => Ok(BMPCompression::RLE4),
            3 => Ok(BMPCompression::BITFIELDS),
            4 => Ok(BMPCompression::JPEG),
            5 => Ok(BMPCompression::PNG),
            _ => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Unsupported compression format: {}", b),
            )),
        }
    }
    pub fn to_bytes(t: &BMPCompression) -> i32 {
        match t {
            &BMPCompression::RGB => 0,
            &BMPCompression::RLE8 => 1,
            &BMPCompression::RLE4 => 2,
            &BMPCompression::BITFIELDS => 3,
            &BMPCompression::JPEG => 4,
            &BMPCompression::PNG => 5,
        }
    }
}

pub const BMP_FILE_HEADER_SIZE: u64 = 14;
pub const BMP_INFO_HEADER_SIZE: i32 = 40;
pub const BMP_V4_INFO_HEADER_SIZE: i32 = 108;
pub const BMP_V5_INFO_HEADER_SIZE: i32 = 124;

#[derive(Debug)]
pub struct Bitmap {
    pub data: Vec<u8>,
    pub decoded_from: Option<BMPCompression>,
}

impl Bitmap {
    pub fn with_capacity(capacity: usize) -> Bitmap {
        Bitmap {
            data: Vec::<u8>::with_capacity(capacity),
            decoded_from: None,
        }
    }
    fn border(&mut self, border_width: i16, width: i32, height: i32, bit_count: i16) {
        let bw = border_width as i32;
        let bc = bit_count as i32;
        // https://en.wikipedia.org/wiki/BMP_file_format
        let bytes_pad = (bc * width + 31)/32*4 - width * bc/8;
        let mut x = 0;
        let mut y = 0;
        let mut rng = rand::thread_rng();
        match bc {
            1 | 4 | 8 =>  {
                let pixels = 8 / bc;
                let max_color = 2u32.pow(bc as u32);
                let mut it = self.data.iter_mut();
                loop {
                    let b;
                    match it.next() {
                        Some(v) => b = v,
                        None => break,
                    }
                    for idx in 1..(pixels+1) {
                        if x <= bw || x > width - bw || y <= bw || y > height - bw {
                            // new colors for border
                            let mut mask = (rng.next_u32() % max_color) as u8;
                            // position in u8 bit fields
                            let shift = (pixels - idx) * bc;
                            // move mask (colors) into appropriate position
                            mask = mask << shift;
                            // clear border pixels (bits)
                            *b = *b & !((max_color-1) << shift) as u8;
                            // apply random border colors (from mask)
                            *b = *b | mask;
                        }
                        x += 1;
                        if x >= width {
                            for _ in 0..bytes_pad {
                                it.next();
                            }
                            x = 0;
                            y += 1;
                            break;
                        }
                    }
                }
            },
            16 => {
                let mut color: u8;
                let max_color = 2u32.pow(5);
                let mut it = self.data.iter_mut();
                let mask: u8 = 0b00011111;
                loop {
                    if let Some(b1) = it.next() {
                        if let Some(b2) = it.next() {
                            if x < bw || x > width - bw || y < bw || y > height - bw {
                                color = (rng.next_u32() % max_color) as u8;
                                *b1 = color & mask;
                                color = (rng.next_u32() % max_color) as u8;
                                *b1 = (*b1 << 2) | ((color & mask) >> 3);
                                *b2 = (color & mask) << 5;
                                color = (rng.next_u32() % max_color) as u8;
                                *b2 = *b2 | (color & mask);
                            }
                            x += 1;
                            if x >= width {
                                for _ in 0..bytes_pad {
                                    it.next();
                                }
                                x = 0;
                                y += 1;
                            }
                            continue;
                        }
                    }
                    break;
                }
            },
            24 => {
                let max_color = std::u16::MAX as u32;
                let mut it = self.data.iter_mut();
                loop {
                    if let Some(b1) = it.next() {
                        if let Some(b2) = it.next() {
                            if let Some(b3) = it.next() {
                                if x < bw || x > width - bw || y < bw || y > height - bw {
                                    *b1 = (rng.next_u32() % max_color) as u8;
                                    *b2 = (rng.next_u32() % max_color) as u8;
                                    *b3 = (rng.next_u32() % max_color) as u8;
                                }
                                x += 1;
                                if x >= width {
                                    for _ in 0..bytes_pad {
                                        it.next();
                                    }
                                    x = 0;
                                    y += 1;
                                }
                                continue;
                            }
                        }
                    }
                    break;
                }
            },
            0 | 32 => unimplemented!("bmp border for bitmap with {} bits per pixel", bc), /* jpeg or png bitmap */
            _ => unreachable!(),
        }
    }

    fn add_logo(&mut self, logo_file: &str, width: i32, height: i32, bit_count: i16) -> io::Result<()> {
        let logo_margin = 15; // pixels
        let bc = bit_count as i32;
        if bc != 24 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Only 24 bits per pixel source BMP are supported.".to_owned(),
            ));
        }

        let logo_info = BMPInfo::load_from_file(logo_file)?;
        let logo_width = logo_info.bmi_header.get_width();
        let logo_height = logo_info.bmi_header.get_height();

        if logo_width > width - logo_margin || logo_height > height - logo_margin {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Logo bitmap too large".to_owned(),
            ));
        }

        let logo_vec = logo_to_rgb_vec(logo_file)?;
        let mut logo_iter = logo_vec.into_iter();

        let mut x = 0;
        let mut x_logo = 0;
        let mut y = 0;
        let mut y_logo = 0;
        let mut it = self.data.iter_mut();
        let bytes_pad = (bc * width + 31)/32*4 - width * bc/8;
        while let (Some(b1), Some(b2), Some(b3)) = (it.next(), it.next(), it.next()) {
            if x >= width - logo_width - logo_margin && y >= height - logo_height - logo_margin {
                if x_logo < logo_width && y_logo < logo_height {
                    if let Some(pixel) = logo_iter.next() {
                        *b1 = pixel.rgb_red;
                        *b2 = pixel.rgb_green;
                        *b3 = pixel.rgb_blue;
                        x_logo += 1;
                    }
                }
            }
            x += 1;
            if x >= width {
                for _ in 0..bytes_pad {
                    it.next();
                }
                x = 0;
                y += 1;
                if x_logo > 0 {
                    x_logo = 0;
                    y_logo += 1;
                }
            }
        }
        Ok(())
    }
}

fn logo_to_rgb_vec(file: &str) -> io::Result<Vec<RGBQuad>> {
    let logo = BMPImage::load_from_file(file)?;
    let bc = logo.info.bmi_header.get_bit_count() as i32;

    if bc != 24 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Only 24 bits per pixel BMP files are supported".to_owned(),
        ));
    }

    let width = logo.info.bmi_header.get_width();
    let height = logo.info.bmi_header.get_height();
    let bytes_pad = (bc * width + 31)/32*4 - width * bc/8;

    let mut it = logo.bitmap.data.iter();

    let mut x = 0;
    let mut result = Vec::with_capacity((width * height) as usize);
    while let (Some(b1), Some(b2), Some(b3)) = (it.next(), it.next(), it.next()) {
        result.push(RGBQuad {
            rgb_red: *b1,
            rgb_green: *b2,
            rgb_blue: *b3,
            rgb_reserved: 0,
        });

        x += 1;
        if x >= width {
            for _ in 0..bytes_pad {
                it.next();
            }
            x = 0;
        }
    }
    Ok(result)
}

#[derive(Debug)]
pub struct BMPImage {
    pub header: BMPFileHeader,
    pub info: BMPInfo,
    pub bitmap: Bitmap,
}

impl BMPImage {
    pub fn meta_from_file<P: AsRef<Path>>(p: P) -> io::Result<BMPImage> {
        let mut f = BufReader::new(File::open(p)?);
        BMPImage::meta_from_reader(&mut f)
    }
    pub fn meta_from_reader<R: ?Sized + BufRead + Seek>(r: &mut R) -> io::Result<BMPImage> {
        let header = BMPFileHeader::load_from_reader(r);
        if header.is_err() {
            let err = header.err().unwrap();
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Unsupported file format: {}", err.description()),
            ));
        }
        let info = BMPInfo::load_from_reader(r);
        if info.is_err() {
            let err = info.err().unwrap();
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to parse metadata: {}", err.description()),
            ));
        }
        Ok(BMPImage {
            header: header.unwrap(),
            info: info.unwrap(),
            bitmap: Bitmap {
                 data: Vec::new(),
                 decoded_from: None,
            },
        })
    }
    pub fn load_from_file<P: AsRef<Path>>(p: P) -> io::Result<BMPImage> {
        let mut f = BufReader::new(File::open(p)?);
        let mut image = BMPImage::meta_from_reader(&mut f)?;
        image.bitmap.data = vec![0u8; image.info.bmi_header.get_bitmap_size() as usize];
        f.read_exact(&mut image.bitmap.data)?;
        Ok(image)
    }
    pub fn grayscale(&mut self) {
        for quad in &mut self.info.bmi_colors {
            let average = (quad.rgb_red as u32 + quad.rgb_green as u32 + quad.rgb_blue as u32) / 3;
            quad.rgb_red = average as u8;
            quad.rgb_green = average as u8;
            quad.rgb_blue = average as u8;
        }
    }
    pub fn border(&mut self, width: i16) {
        self.decode_bitmap();
        self.bitmap.border(
            width,
            self.info.bmi_header.get_width(),
            self.info.bmi_header.get_height(),
            self.info.bmi_header.get_bit_count(),
        );
        /*
        if let Some(enc) = self.bitmap.decoded_from {
            // if we decode bitmap encode it into source encoding
            self.encode_bitmap(enc);
        }
        */
    }
    pub fn add_logo(&mut self, logo: &str) {
        self.bitmap.add_logo(
            logo,
            self.info.bmi_header.get_width(),
            self.info.bmi_header.get_height(),
            self.info.bmi_header.get_bit_count(),
        ).unwrap_or_else(|e| {
            eprintln!("Can't add logo: {}", e);
            process::exit(1);
        });
    }

    pub fn save_to_file<P: AsRef<Path>>(&mut self, p: P) -> io::Result<usize> {
        let mut f = BufWriter::new(File::create(p)?);
        self.header.save_to_writer(&mut f)?;
        self.info.save_to_writer(&mut f)?;
        f.write_all(&self.bitmap.data)?;
        Ok(0 as usize)
    }

    pub fn encode_bitmap(&mut self, compression: BMPCompression) {
        let width = self.info.bmi_header.get_width();
        let height = self.info.bmi_header.get_height();
        match compression {
            BMPCompression::RGB => return,
            BMPCompression::RLE8 => Rle8::encode(&mut self.bitmap, width, height),
            BMPCompression::RLE4 => unimplemented!("bmp Rle4 bitmap encoding"),
            BMPCompression::BITFIELDS => unimplemented!("bmp bitfield bitmap encoding"),
            BMPCompression::JPEG => unimplemented!("bmp JPEG bitmap encoding"),
            BMPCompression::PNG => unimplemented!("bmp PNG bitmap encoding"),
        };
    }
    pub fn decode_bitmap(&mut self) {
        let width = self.info.bmi_header.get_width();
        let height = self.info.bmi_header.get_height();
        match self.info.bmi_header.get_compression_type() {
            BMPCompression::RGB => return,
            BMPCompression::RLE8 => Rle8::decode(&mut self.bitmap, width, height),
            BMPCompression::RLE4 => unimplemented!("bmp Rle4 bitmap decoding"),
            BMPCompression::BITFIELDS => unimplemented!("bmp bitfields decoding"),
            BMPCompression::JPEG => unimplemented!("bmp JPEG bitmap decoding"),
            BMPCompression::PNG => unimplemented!("bmp PNG bitmap decoding"),
        };
        self.info.bmi_header.set_encoding(BMPCompression::RGB);

        let delta = self.bitmap.data.len() as i32 - self.info.bmi_header.get_bitmap_size();
        match self.info.bmi_header {
            BMPGenericInfoHeader::Info(ref mut i) => i.bi_size_image += delta,
            BMPGenericInfoHeader::V4Info(ref mut i) => i.bv4_size_image += delta,
            BMPGenericInfoHeader::V5Info(ref mut i) => i.bv5_size_image += delta,
        }
        self.header.bf_size += delta;
    }
}

impl fmt::Display for BMPImage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.header.fmt(f)?;
        self.info.fmt(f)
    }
}

#[derive(Debug)]
pub struct BMPFileHeader {
    /// The file type; must be BM
    bf_type: i16,
    /// The size, in bytes, of the bitmap file
    bf_size: i32,
    /// Reserved; must be zero
    bf_reserved1: i16,
    /// Reserved; must be zero
    bf_reserved2: i16,
    /// The offset, in bytes, from the beginning of
    /// the BITMAPFILEHEADER structure to the bitmap bits
    bf_offset_bits: i32,
}

impl fmt::Display for BMPFileHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Size: {} bytes\n", self.bf_size)
    }
}

impl BMPFileHeader {
    pub fn new(size: i32, offset: i32) -> BMPFileHeader {
        BMPFileHeader {
            bf_type: /* LittleEndian */ 'B' as i16 + (('M' as i16) << 8),
            bf_size: size,
            bf_reserved1: 0,
            bf_reserved2: 0,
            bf_offset_bits: offset,
        }
    }
    pub fn load_from_file<P: AsRef<Path>>(p: P) -> io::Result<BMPFileHeader> {
        let mut f = BufReader::new(File::open(p)?);
        BMPFileHeader::load_from_reader(&mut f)
    }
    pub fn load_from_reader<R: ?Sized + BufRead>(r: &mut R) -> io::Result<BMPFileHeader> {
        let mut sig = [0u8; 2];
        try!(r.read_exact(&mut sig));
        if sig != b"BM"[..] {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid BMP signature: '{:?}'", sig),
            ));
        }
        Ok(BMPFileHeader {
            bf_type: ((sig[1] as i16) << 8) + (sig[0] as i16),
            bf_size: r.read_i32::<LittleEndian>()?,
            bf_reserved1: r.read_i16::<LittleEndian>()?,
            bf_reserved2: r.read_i16::<LittleEndian>()?,
            bf_offset_bits: r.read_i32::<LittleEndian>()?,
        })
    }

    pub fn save_to_writer<W: ?Sized + Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_i16::<LittleEndian>(self.bf_type)?;
        w.write_i32::<LittleEndian>(self.bf_size)?;
        w.write_i32::<LittleEndian>(0i32)?; // reserved1 0_i16 and reserved2 0_i16
        w.write_i32::<LittleEndian>(self.bf_offset_bits)?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum BMPGenericInfoHeader {
    Info(BMPInfoHeader),
    V4Info(BMPV4Header),
    V5Info(BMPV5Header),
}

impl BMPGenericInfoHeader {
    pub fn get_width(&self) -> i32 {
        match self {
            &BMPGenericInfoHeader::Info(ref i) => i.bi_width,
            &BMPGenericInfoHeader::V4Info(ref i) => i.bv4_width,
            &BMPGenericInfoHeader::V5Info(ref i) => i.bv5_width,
        }
    }
    pub fn get_height(&self) -> i32 {
        match self {
            &BMPGenericInfoHeader::Info(ref i) => i.bi_height,
            &BMPGenericInfoHeader::V4Info(ref i) => i.bv4_height,
            &BMPGenericInfoHeader::V5Info(ref i) => i.bv5_height,
        }
    }
    pub fn get_bit_count(&self) -> i16 {
        match self {
            &BMPGenericInfoHeader::Info(ref i) => i.bi_bit_count,
            &BMPGenericInfoHeader::V4Info(ref i) => i.bv4_bit_count,
            &BMPGenericInfoHeader::V5Info(ref i) => i.bv5_bit_count,
        }
    }
    pub fn get_compression_type(&self) -> BMPCompression {
        match self {
            &BMPGenericInfoHeader::Info(ref i) => i.bi_compression,
            &BMPGenericInfoHeader::V4Info(ref i) => i.bv4_v4_compression,
            &BMPGenericInfoHeader::V5Info(ref i) => i.bv5_compression,
        }
    }
    pub fn get_colors_used(&self) -> i32 {
        match self {
            &BMPGenericInfoHeader::Info(ref i) => i.bi_clr_used,
            &BMPGenericInfoHeader::V4Info(ref i) => i.bv4_clr_used,
            &BMPGenericInfoHeader::V5Info(ref i) => i.bv5_clr_used,
        }
    }
    pub fn get_bitmap_size(&self) -> i32 {
        let mut size = match self {
            &BMPGenericInfoHeader::Info(ref i) => i.bi_size_image,
            &BMPGenericInfoHeader::V4Info(ref i) => i.bv4_size_image,
            &BMPGenericInfoHeader::V5Info(ref i) => i.bv5_size_image,
        };
        if size == 0 {
            size = self.get_width() * self.get_height() * self.get_bit_count() as i32 / 8
        };
        size
    }
    pub fn get_type(&self) -> &'static str {
        match self {
            &BMPGenericInfoHeader::Info(_) => "BMPInfoHeader",
            &BMPGenericInfoHeader::V4Info(_) => "BMPV4Header",
            &BMPGenericInfoHeader::V5Info(_) => "BMPV5Header",
        }
    }
    pub fn get_os_support(&self) -> &'static str {
        match *self {
            BMPGenericInfoHeader::Info(_) => "Windows NT, 3.1x or later",
            BMPGenericInfoHeader::V4Info(_) => "Windows NT 4.0, 95 or later",
            BMPGenericInfoHeader::V5Info(_) => "Windows NT 5.0, 98 or later",
        }
    }
    pub fn set_encoding(&mut self, enc :BMPCompression) {
        match *self {
            BMPGenericInfoHeader::Info(ref mut i) => i.bi_compression = enc,
            BMPGenericInfoHeader::V4Info(ref mut i) => i.bv4_v4_compression = enc,
            BMPGenericInfoHeader::V5Info(ref mut i) => i.bv5_compression = enc,
        }
    }
}

#[derive(Debug)]
pub struct BMPInfo {
    /// A BITMAPINFOHEADER structure that contains information about the dimensions of color format.
    pub bmi_header: BMPGenericInfoHeader,
    /// An array of RGBQUAD. The elements of the array that make up the color table.
    pub bmi_colors: Vec<RGBQuad>,
}

impl BMPInfo {
    pub fn load_from_file<P: AsRef<Path>>(p: P) -> io::Result<BMPInfo> {
        let mut f = BufReader::new(File::open(p)?);
        BMPInfo::load_from_reader(&mut f)
    }
    pub fn load_from_reader<R: ?Sized + BufRead + Seek>(r: &mut R) -> io::Result<BMPInfo> {
        // skip file header
        r.seek(SeekFrom::Start(BMP_FILE_HEADER_SIZE))?;
        let size = r.read_i32::<LittleEndian>()?;
        r.seek(SeekFrom::Start(BMP_FILE_HEADER_SIZE))?;

        let header = match size {
            BMP_INFO_HEADER_SIZE => BMPGenericInfoHeader::Info(BMPInfoHeader::load_from_reader(r)?),
            BMP_V4_INFO_HEADER_SIZE => {
                BMPGenericInfoHeader::V4Info(BMPV4Header::load_from_reader(r)?)
            }
            BMP_V5_INFO_HEADER_SIZE => {
                BMPGenericInfoHeader::V5Info(BMPV5Header::load_from_reader(r)?)
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Unknown info header, size={} bytes", size),
                ));
            }
        };
        let mut colors = Vec::<RGBQuad>::new();
        if header.get_bit_count() < 16 {
            let palette_len = 2u64.pow(header.get_bit_count() as u32);
            for _ in 0..palette_len {
                colors.push(RGBQuad::load_from_reader(r)?);
            }
        }
        Ok(BMPInfo {
            bmi_header: header,
            bmi_colors: colors,
        })
    }
    pub fn save_to_writer<W: ?Sized + Write>(&self, w: &mut W) -> io::Result<()> {
        match self.bmi_header {
            BMPGenericInfoHeader::Info(ref info) => info.save_to_writer(w)?,
            BMPGenericInfoHeader::V4Info(ref info) => info.save_to_writer(w)?,
            BMPGenericInfoHeader::V5Info(ref info) => info.save_to_writer(w)?,
        };
        for c in &self.bmi_colors {
            c.save_to_writer(w)?;
        }
        Ok(())
    }
}

impl fmt::Display for BMPInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Info type: {}\n\
             OS support: {}\n\
             Compression: {}\n\
             Width: {} px\nHeight: {} px\n\
             Bit Per Pixel: {}\n\
             {}\
             Max Colors: {}",
            self.bmi_header.get_type(),
            self.bmi_header.get_os_support(),
            self.bmi_header.get_compression_type(),
            self.bmi_header.get_width(),
            self.bmi_header.get_height(),
            self.bmi_header.get_bit_count(),
            if self.bmi_header.get_bit_count() < 16 {
                format!("Colors used: {}\n", self.bmi_header.get_colors_used())
            } else {
                String::new()
            },
            2u64.pow(self.bmi_header.get_bit_count() as u32),
        )
    }
}

#[derive(Debug)]
pub struct BMPInfoHeader {
    /// The number of bytes required by this structure
    bi_size: i32,
    /// The width of the bitmap, in pixels
    bi_width: i32,
    /// The height of the bitmap, in pixels
    bi_height: i32,
    /// The number of planes for the target device. This value must be set to 1
    bi_planes: i16,
    /// The number of bits-per-pixel
    bi_bit_count: i16,
    /// The type of compression for a compressed bottom-up bitmap
    bi_compression: BMPCompression,
    /// The size, in bytes, of the image
    bi_size_image: i32,
    /// The horizontal resolution, in pixels-per-meter
    bi_x_pels_per_meter: i32,
    /// The vertical resolution, in pixels-per-meter
    bi_y_pels_per_meter: i32,
    /// The number of color indexes in the color table that are actually used by the bitmap
    bi_clr_used: i32,
    /// The number of color indexes that are required for displaying the bitmap
    bi_clr_important: i32,
}
impl BMPInfoHeader {
    pub fn new(
        width: i32, height: i32,
        bpp: i16, size: i32,
        x_ppm: i32, y_ppm: i32,
        clr_used: i32,
        clr_important: i32,
        ) -> BMPInfoHeader {
        BMPInfoHeader{
            bi_size: 40,
            bi_width: width,
            bi_height: height,
            bi_planes: 1,
            bi_bit_count: bpp,
            bi_compression: BMPCompression::RGB,
            bi_size_image: size,
            bi_x_pels_per_meter: x_ppm,
            bi_y_pels_per_meter: y_ppm,
            bi_clr_used: clr_used,
            bi_clr_important: clr_important,
        }
    }
    pub fn load_from_reader<R: ?Sized + BufRead>(r: &mut R) -> io::Result<BMPInfoHeader> {
        Ok(BMPInfoHeader {
            bi_size: r.read_i32::<LittleEndian>()?,
            bi_width: r.read_i32::<LittleEndian>()?,
            bi_height: r.read_i32::<LittleEndian>()?,
            bi_planes: r.read_i16::<LittleEndian>()?,
            bi_bit_count: r.read_i16::<LittleEndian>()?,
            bi_compression: BMPCompression::from_bytes(r.read_i32::<LittleEndian>()?)?,
            bi_size_image: r.read_i32::<LittleEndian>()?,
            bi_x_pels_per_meter: r.read_i32::<LittleEndian>()?,
            bi_y_pels_per_meter: r.read_i32::<LittleEndian>()?,
            bi_clr_used: r.read_i32::<LittleEndian>()?,
            bi_clr_important: r.read_i32::<LittleEndian>()?,
        })
    }
    pub fn save_to_writer<W: ?Sized + Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_i32::<LittleEndian>(self.bi_size)?;
        w.write_i32::<LittleEndian>(self.bi_width)?;
        w.write_i32::<LittleEndian>(self.bi_height)?;
        w.write_i16::<LittleEndian>(self.bi_planes)?;
        w.write_i16::<LittleEndian>(self.bi_bit_count)?;
        w.write_i32::<LittleEndian>(BMPCompression::to_bytes(&self.bi_compression))?;
        w.write_i32::<LittleEndian>(self.bi_size_image)?;
        w.write_i32::<LittleEndian>(self.bi_x_pels_per_meter)?;
        w.write_i32::<LittleEndian>(self.bi_y_pels_per_meter)?;
        w.write_i32::<LittleEndian>(self.bi_clr_used)?;
        w.write_i32::<LittleEndian>(self.bi_clr_important)?;
        Ok(())
    }
}

/// In all versions of BMP files starting with Version 3 (Win3x),
/// the color entries occupy 4 bytes each so that they can be efficiently
/// read and written as single 32-bit values. Taken as a single value,
/// the four bytes are ordered as follows: [ZERO][RED][GREEN][BLUE].
/// Due to the Little Endian format, this means that the Blue value comes first
/// followed by the green and then the red.
/// A fourth, unused, byte comes next which is expected to be equal to 0.
#[derive(Debug, Copy, Clone)]
pub struct RGBQuad {
    rgb_blue: u8,
    rgb_green: u8,
    rgb_red: u8,
    /// This member is reserved and must be zero
    rgb_reserved: u8,
}

impl RGBQuad {
    pub fn new(red: u8, green: u8, blue: u8) -> RGBQuad {
        RGBQuad {
            rgb_red: red,
            rgb_green: green,
            rgb_blue: blue,
            rgb_reserved: 0,
        }
    }
    pub fn change(&mut self, red: u8, green: u8, blue: u8) {
            self.rgb_red = red;
            self.rgb_green = green;
            self.rgb_blue = blue;
    }

    pub fn load_from_reader<R: ?Sized + BufRead>(r: &mut R) -> io::Result<RGBQuad> {
        Ok(RGBQuad {
            rgb_blue: r.read_u8()?,
            rgb_green: r.read_u8()?,
            rgb_red: r.read_u8()?,
            rgb_reserved: r.read_u8()?,
        })
    }
    pub fn save_to_writer<W: ?Sized + Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_u8(self.rgb_blue)?;
        w.write_u8(self.rgb_green)?;
        w.write_u8(self.rgb_red)?;
        w.write_u8(self.rgb_reserved)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct BMPV4Header {
    bv4_size: i32,
    bv4_width: i32,
    bv4_height: i32,
    bv4_planes: i16,
    bv4_bit_count: i16,
    bv4_v4_compression: BMPCompression,
    bv4_size_image: i32,
    bv4_x_pels_per_meter: i32,
    bv4_y_pels_per_meter: i32,
    bv4_clr_used: i32,
    bv4_clr_important: i32,
    bv4_red_mask: i32,
    bv4_green_mask: i32,
    bv4_blue_mask: i32,
    bv4_alpha_mask: i32,
    bv4_cs_type: i32,
    bv4_endpoints: CIEXYZTriple,
    bv4_gamma_red: i32,
    bv4_gamma_green: i32,
    bv4_gamma_blue: i32,
}

impl BMPV4Header {
    pub fn load_from_reader<R: ?Sized + BufRead>(r: &mut R) -> io::Result<BMPV4Header> {
        Ok(BMPV4Header {
            bv4_size: r.read_i32::<LittleEndian>()?,
            bv4_width: r.read_i32::<LittleEndian>()?,
            bv4_height: r.read_i32::<LittleEndian>()?,
            bv4_planes: r.read_i16::<LittleEndian>()?,
            bv4_bit_count: r.read_i16::<LittleEndian>()?,
            bv4_v4_compression: BMPCompression::from_bytes(r.read_i32::<LittleEndian>()?)?,
            bv4_size_image: r.read_i32::<LittleEndian>()?,
            bv4_x_pels_per_meter: r.read_i32::<LittleEndian>()?,
            bv4_y_pels_per_meter: r.read_i32::<LittleEndian>()?,
            bv4_clr_used: r.read_i32::<LittleEndian>()?,
            bv4_clr_important: r.read_i32::<LittleEndian>()?,
            bv4_red_mask: r.read_i32::<LittleEndian>()?,
            bv4_green_mask: r.read_i32::<LittleEndian>()?,
            bv4_blue_mask: r.read_i32::<LittleEndian>()?,
            bv4_alpha_mask: r.read_i32::<LittleEndian>()?,
            bv4_cs_type: r.read_i32::<LittleEndian>()?,
            bv4_endpoints: CIEXYZTriple::load_from_reader(r)?,
            bv4_gamma_red: r.read_i32::<LittleEndian>()?,
            bv4_gamma_green: r.read_i32::<LittleEndian>()?,
            bv4_gamma_blue: r.read_i32::<LittleEndian>()?,
        })
    }
    pub fn save_to_writer<W: ?Sized + Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_i32::<LittleEndian>(self.bv4_size)?;
        w.write_i32::<LittleEndian>(self.bv4_width)?;
        w.write_i32::<LittleEndian>(self.bv4_height)?;
        w.write_i16::<LittleEndian>(self.bv4_planes)?;
        w.write_i16::<LittleEndian>(self.bv4_bit_count)?;
        w.write_i32::<LittleEndian>(BMPCompression::to_bytes(&self.bv4_v4_compression))?;
        w.write_i32::<LittleEndian>(self.bv4_size_image)?;
        w.write_i32::<LittleEndian>(self.bv4_x_pels_per_meter)?;
        w.write_i32::<LittleEndian>(self.bv4_y_pels_per_meter)?;
        w.write_i32::<LittleEndian>(self.bv4_clr_used)?;
        w.write_i32::<LittleEndian>(self.bv4_clr_important)?;
        w.write_i32::<LittleEndian>(self.bv4_red_mask)?;
        w.write_i32::<LittleEndian>(self.bv4_green_mask)?;
        w.write_i32::<LittleEndian>(self.bv4_blue_mask)?;
        w.write_i32::<LittleEndian>(self.bv4_alpha_mask)?;
        w.write_i32::<LittleEndian>(self.bv4_cs_type)?;
        self.bv4_endpoints.save_to_writer(w)?;
        w.write_i32::<LittleEndian>(self.bv4_gamma_red)?;
        w.write_i32::<LittleEndian>(self.bv4_gamma_green)?;
        w.write_i32::<LittleEndian>(self.bv4_gamma_blue)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct CIEXYZTriple {
    ciexyz_red: CIEXYZ,
    ciexyz_green: CIEXYZ,
    ciexyz_blue: CIEXYZ,
}
impl CIEXYZTriple {
    pub fn load_from_reader<R: ?Sized + BufRead>(r: &mut R) -> io::Result<CIEXYZTriple> {
        Ok(CIEXYZTriple {
            ciexyz_red: CIEXYZ::load_from_reader(r)?,
            ciexyz_green: CIEXYZ::load_from_reader(r)?,
            ciexyz_blue: CIEXYZ::load_from_reader(r)?,
        })
    }
    pub fn save_to_writer<W: ?Sized + Write>(&self, w: &mut W) -> io::Result<()> {
        self.ciexyz_red.save_to_writer(w)?;
        self.ciexyz_green.save_to_writer(w)?;
        self.ciexyz_blue.save_to_writer(w)?;
        Ok(())
    }
}

type Fxpt2Dot30 = u32;
#[derive(Debug)]
pub struct CIEXYZ {
    ciexyz_x: Fxpt2Dot30,
    ciexyz_y: Fxpt2Dot30,
    ciexyz_z: Fxpt2Dot30,
}

impl CIEXYZ {
    pub fn load_from_reader<R: ?Sized + BufRead>(r: &mut R) -> io::Result<CIEXYZ> {
        Ok(CIEXYZ {
            ciexyz_x: r.read_u32::<LittleEndian>()?,
            ciexyz_y: r.read_u32::<LittleEndian>()?,
            ciexyz_z: r.read_u32::<LittleEndian>()?,
        })
    }
    pub fn save_to_writer<W: ?Sized + Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_u32::<LittleEndian>(self.ciexyz_x)?;
        w.write_u32::<LittleEndian>(self.ciexyz_y)?;
        w.write_u32::<LittleEndian>(self.ciexyz_z)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct BMPV5Header {
    bv5_size: i32,
    bv5_width: i32,
    bv5_height: i32,
    bv5_planes: i16,
    bv5_bit_count: i16,
    bv5_compression: BMPCompression,
    bv5_size_image: i32,
    bv5_x_pels_per_meter: i32,
    bv5_y_pels_per_meter: i32,
    bv5_clr_used: i32,
    bv5_clr_important: i32,
    bv5_red_mask: i32,
    bv5_green_mask: i32,
    bv5_blue_mask: i32,
    bv5_alpha_mask: i32,
    bv5_cs_type: i32,
    bv5_endpoints: CIEXYZTriple,
    bv5_gamma_red: i32,
    bv5_gamma_green: i32,
    bv5_gamma_blue: i32,
    bv5_intent: i32,
    bv5_profile_data: i32,
    bv5_profile_size: i32,
    bv5_reserved: i32,
}

impl BMPV5Header {
    pub fn load_from_reader<R: ?Sized + BufRead>(r: &mut R) -> io::Result<BMPV5Header> {
        Ok(BMPV5Header {
            bv5_size: r.read_i32::<LittleEndian>()?,
            bv5_width: r.read_i32::<LittleEndian>()?,
            bv5_height: r.read_i32::<LittleEndian>()?,
            bv5_planes: r.read_i16::<LittleEndian>()?,
            bv5_bit_count: r.read_i16::<LittleEndian>()?,
            bv5_compression: BMPCompression::from_bytes(r.read_i32::<LittleEndian>()?)?,
            bv5_size_image: r.read_i32::<LittleEndian>()?,
            bv5_x_pels_per_meter: r.read_i32::<LittleEndian>()?,
            bv5_y_pels_per_meter: r.read_i32::<LittleEndian>()?,
            bv5_clr_used: r.read_i32::<LittleEndian>()?,
            bv5_clr_important: r.read_i32::<LittleEndian>()?,
            bv5_red_mask: r.read_i32::<LittleEndian>()?,
            bv5_green_mask: r.read_i32::<LittleEndian>()?,
            bv5_blue_mask: r.read_i32::<LittleEndian>()?,
            bv5_alpha_mask: r.read_i32::<LittleEndian>()?,
            bv5_cs_type: r.read_i32::<LittleEndian>()?,
            bv5_endpoints: CIEXYZTriple::load_from_reader(r)?,
            bv5_gamma_red: r.read_i32::<LittleEndian>()?,
            bv5_gamma_green: r.read_i32::<LittleEndian>()?,
            bv5_gamma_blue: r.read_i32::<LittleEndian>()?,
            bv5_intent: r.read_i32::<LittleEndian>()?,
            bv5_profile_data: r.read_i32::<LittleEndian>()?,
            bv5_profile_size: r.read_i32::<LittleEndian>()?,
            bv5_reserved: r.read_i32::<LittleEndian>()?,
        })
    }
    pub fn save_to_writer<W: ?Sized + Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_i32::<LittleEndian>(self.bv5_size)?;
        w.write_i32::<LittleEndian>(self.bv5_width)?;
        w.write_i32::<LittleEndian>(self.bv5_height)?;
        w.write_i16::<LittleEndian>(self.bv5_planes)?;
        w.write_i16::<LittleEndian>(self.bv5_bit_count)?;
        w.write_i32::<LittleEndian>(BMPCompression::to_bytes(&self.bv5_compression))?;
        w.write_i32::<LittleEndian>(self.bv5_size_image)?;
        w.write_i32::<LittleEndian>(self.bv5_x_pels_per_meter)?;
        w.write_i32::<LittleEndian>(self.bv5_y_pels_per_meter)?;
        w.write_i32::<LittleEndian>(self.bv5_clr_used)?;
        w.write_i32::<LittleEndian>(self.bv5_clr_important)?;
        w.write_i32::<LittleEndian>(self.bv5_red_mask)?;
        w.write_i32::<LittleEndian>(self.bv5_green_mask)?;
        w.write_i32::<LittleEndian>(self.bv5_blue_mask)?;
        w.write_i32::<LittleEndian>(self.bv5_alpha_mask)?;
        w.write_i32::<LittleEndian>(self.bv5_cs_type)?;
        self.bv5_endpoints.save_to_writer(w)?;
        w.write_i32::<LittleEndian>(self.bv5_gamma_red)?;
        w.write_i32::<LittleEndian>(self.bv5_gamma_green)?;
        w.write_i32::<LittleEndian>(self.bv5_gamma_blue)?;
        w.write_i32::<LittleEndian>(self.bv5_intent)?;
        w.write_i32::<LittleEndian>(self.bv5_profile_data)?;
        w.write_i32::<LittleEndian>(self.bv5_profile_size)?;
        w.write_i32::<LittleEndian>(self.bv5_reserved)?;
        Ok(())
    }
}
