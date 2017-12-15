//! Device-independent bitmaps
//! The format for a DIB is the following (for more information, see Bitmap Storage ):
//!   * a BITMAPFILEHEADER structure
//!   * either a BITMAPCOREHEADER, a BITMAPINFOHEADER, a BITMAPV4HEADER, or a BITMAPV5HEADER structure.
//!   * an optional color table, which is either a set of RGBQUAD structures or a set of RGBTRIPLE structures.
//!   * the bitmap data
//!   * optional Profile data
//! A color table describes how pixel values correspond to RGB color values.
//! RGB is a model for describing colors that are produced by emitting light.
//!
//! The four types of bitmap headers are differentiated by the Size member,
//! which is the first DWORD in each of the structures.
//!
//! see https://msdn.microsoft.com/en-us/library/dd183386(v=vs.85).aspx
//! and https://msdn.microsoft.com/en-us/library/dd183391(v=vs.85).aspx
//! for more info

use std::fs::File;
use std::path::Path;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use byteorder::{LittleEndian, ReadBytesExt};

/// An uncompressed format.
pub const BI_RGB: i16 = 0;
/// A run-length encoded (RLE) format for bitmaps with 8 bpp.
/// The compression format is a 2-byte format consisting of a count byte
/// followed by a byte containing a color index. For more information.
pub const BI_RLE8: i16 = 1;
/// An RLE format for bitmaps with 4 bpp.
/// The compression format is a 2-byte format consisting of a count byte
/// followed by two word-length color indexes. For more information.
pub const BI_RLE4: i16 = 2;
/// Specifies that the bitmap is not compressed and that the color table
/// consists of three DWORD color masks that specify
/// the red, green, and blue components, respectively, of each pixel.
/// This is valid when used with 16- and 32-bpp bitmaps.
pub const BI_BITFIELDS: i16 = 3;
/// Indicates that the image is a JPEG image.
pub const BI_JPEG: i16 = 4;
///  Indicates that the image is a PNG image.
pub const BI_PNG: i16 = 5;

pub const BMP_FILE_HEADER_SIZE: u64 = 14;
pub const BMP_CORE_INFO_HEADER_SIZE: i32 = 12;
pub const BMP_INFO_HEADER_SIZE: i32 = 40;
pub const BMP_V4_INFO_HEADER_SIZE: i32 = 104;
pub const BMP_V5_INFO_HEADER_SIZE: i32 = 124;

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

impl BMPFileHeader {
    pub fn load_from_file<P: AsRef<Path>>(p: P) -> Result<BMPFileHeader, io::Error> {
        let mut f = BufReader::new(File::open(p)?);
        let mut sig = [0u8; 2];
        try!(f.read_exact(&mut sig));
        if &sig != b"BM" {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Invalid BMP signature: '{:?}'", sig),
            ));
        }
        Ok(BMPFileHeader {
            bf_type: ((sig[0] as i16) << 8) + (sig[1] as i16),
            bf_size: f.read_i32::<LittleEndian>()?,
            bf_reserved1: f.read_i16::<LittleEndian>()?,
            bf_reserved2: f.read_i16::<LittleEndian>()?,
            bf_offset_bits: f.read_i32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug)]
pub enum BMPGenericInfo {
    CoreInfo(BMPCoreInfo),
    Info(BMPInfo),
}

impl BMPGenericInfo {
    pub fn load_from_file<P: AsRef<Path>>(p: P) -> Result<BMPGenericInfo, io::Error> {
        let mut f = BufReader::new(File::open(p)?);
        // skip file header
        f.seek(SeekFrom::Start(BMP_FILE_HEADER_SIZE))?;
        let size = f.read_i32::<LittleEndian>()?;
        f.seek(SeekFrom::Start(BMP_FILE_HEADER_SIZE))?;

        if size == BMP_CORE_INFO_HEADER_SIZE {
            return Ok(BMPGenericInfo::CoreInfo(BMPCoreInfo {
                bmci_header: BMPCoreHeader::load_from_reader(&mut f)?,
                bmci_colors: Vec::<RGBTriple>::new(),
            }));
        } else if size == BMP_INFO_HEADER_SIZE {
            return Ok(BMPGenericInfo::Info(BMPInfo {
                bmi_header: BMPGenericInfoHeader::Info(BMPInfoHeader::load_from_reader(&mut f)?),
                bmi_colors: Vec::<RGBQuad>::new(),
            }));
        } else if size == BMP_V4_INFO_HEADER_SIZE {
            return Ok(BMPGenericInfo::Info(BMPInfo {
                bmi_header: BMPGenericInfoHeader::V4Info(BMPV4Header::load_from_reader(&mut f)?),
                bmi_colors: Vec::<RGBQuad>::new(),
            }));
        } else if size == BMP_V5_INFO_HEADER_SIZE {
            return Ok(BMPGenericInfo::Info(BMPInfo {
                bmi_header: BMPGenericInfoHeader::V5Info(BMPV5Header::load_from_reader(&mut f)?),
                bmi_colors: Vec::<RGBQuad>::new(),
            }));
        }
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Unknown info header, size={} bytes", size),
        ))
    }
}

#[derive(Debug)]
pub struct BMPCoreInfo {
    /// A BITMAPCOREHEADER structure that contains information about the dimensions and color format of a DIB.
    bmci_header: BMPCoreHeader,
    /// Specifies an array of RGBTRIPLE structures that define the colors in the bitmap
    bmci_colors: Vec<RGBTriple>,
}

#[derive(Debug)]
pub struct BMPCoreHeader {
    /// The number of bytes required by the structure
    bc_size: i32,
    /// The width of the bitmap, in pixels
    bc_width: i16,
    /// The height of the bitmap, in pixels
    bc_height: i16,
    /// The number of planes for the target device. This value must be 1
    bc_planes: i16,
    /// The number of bits-per-pixel. This value must be 1, 4, 8, or 24.
    bc_bit_count: i16,
}

impl BMPCoreHeader {
    pub fn load_from_reader<R: ?Sized + BufRead>(r: &mut R) -> Result<BMPCoreHeader, io::Error> {
        Ok(BMPCoreHeader {
            bc_size: r.read_i32::<LittleEndian>()?,
            bc_width: r.read_i16::<LittleEndian>()?,
            bc_height: r.read_i16::<LittleEndian>()?,
            bc_planes: r.read_i16::<LittleEndian>()?,
            bc_bit_count: r.read_i16::<LittleEndian>()?,
        })
    }
}

#[derive(Debug)]
pub struct RGBTriple {
    rgbt_blue: u8,
    rgbt_green: u8,
    rgbt_red: u8,
}

#[derive(Debug)]
pub enum BMPGenericInfoHeader {
    Info(BMPInfoHeader),
    V4Info(BMPV4Header),
    V5Info(BMPV5Header),
}

#[derive(Debug)]
pub struct BMPInfo {
    /// A BITMAPINFOHEADER structure that contains information about the dimensions of color format.
    bmi_header: BMPGenericInfoHeader,
    /// An array of RGBQUAD. The elements of the array that make up the color table.
    bmi_colors: Vec<RGBQuad>,
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
    bi_compression: i32,
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
    pub fn load_from_reader<R: ?Sized + BufRead>(r: &mut R) -> Result<BMPInfoHeader, io::Error> {
        Ok(BMPInfoHeader {
            bi_size: r.read_i32::<LittleEndian>()?,
            bi_width: r.read_i32::<LittleEndian>()?,
            bi_height: r.read_i32::<LittleEndian>()?,
            bi_planes: r.read_i16::<LittleEndian>()?,
            bi_bit_count: r.read_i16::<LittleEndian>()?,
            bi_compression: r.read_i32::<LittleEndian>()?,
            bi_size_image: r.read_i32::<LittleEndian>()?,
            bi_x_pels_per_meter: r.read_i32::<LittleEndian>()?,
            bi_y_pels_per_meter: r.read_i32::<LittleEndian>()?,
            bi_clr_used: r.read_i32::<LittleEndian>()?,
            bi_clr_important: r.read_i32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug)]
pub struct RGBQuad {
    rgb_blue: u8,
    rgb_green: u8,
    rgb_red: u8,
    /// This member is reserved and must be zero
    rgb_reserved: u8,
}

#[derive(Debug)]
pub struct BMPV4Header {
    bv4_size: i32,
    bv4_width: i32,
    bv4_height: i32,
    bv4_planes: i16,
    bv4_bit_count: i16,
    bv4_v4_compression: i32,
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
    pub fn load_from_reader<R: ?Sized + BufRead>(r: &mut R) -> Result<BMPV4Header, io::Error> {
        Ok(BMPV4Header {
            bv4_size: r.read_i32::<LittleEndian>()?,
            bv4_width: r.read_i32::<LittleEndian>()?,
            bv4_height: r.read_i32::<LittleEndian>()?,
            bv4_planes: r.read_i16::<LittleEndian>()?,
            bv4_bit_count: r.read_i16::<LittleEndian>()?,
            bv4_v4_compression: r.read_i32::<LittleEndian>()?,
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
}

#[derive(Debug)]
pub struct CIEXYZTriple {
    ciexyz_red: CIEXYZ,
    ciexyz_green: CIEXYZ,
    ciexyz_blue: CIEXYZ,
}
impl CIEXYZTriple {
    pub fn load_from_reader<R: ?Sized + BufRead>(r: &mut R) -> Result<CIEXYZTriple, io::Error> {
        Ok(CIEXYZTriple {
            ciexyz_red: CIEXYZ::load_from_reader(r)?,
            ciexyz_green: CIEXYZ::load_from_reader(r)?,
            ciexyz_blue: CIEXYZ::load_from_reader(r)?,
        })
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
    pub fn load_from_reader<R: ?Sized + BufRead>(r: &mut R) -> Result<CIEXYZ, io::Error> {
        Ok(CIEXYZ {
            ciexyz_x: r.read_u32::<LittleEndian>()?,
            ciexyz_y: r.read_u32::<LittleEndian>()?,
            ciexyz_z: r.read_u32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug)]
pub struct BMPV5Header {
    bv5_size: i32,
    bv5_width: i32,
    bv5_height: i32,
    bv5_planes: i16,
    bv5_bit_count: i16,
    bv5_compression: i32,
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
    pub fn load_from_reader<R: ?Sized + BufRead>(r: &mut R) -> Result<BMPV5Header, io::Error> {
        Ok(BMPV5Header {
            bv5_size: r.read_i32::<LittleEndian>()?,
            bv5_width: r.read_i32::<LittleEndian>()?,
            bv5_height: r.read_i32::<LittleEndian>()?,
            bv5_planes: r.read_i16::<LittleEndian>()?,
            bv5_bit_count: r.read_i16::<LittleEndian>()?,
            bv5_compression: r.read_i32::<LittleEndian>()?,
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
}
