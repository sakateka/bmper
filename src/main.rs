pub mod bmp {
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
        pub fn new() -> BMPFileHeader {
            BMPFileHeader {
                bf_type: 0,
                bf_size: 0,
                bf_reserved1: 0,
                bf_reserved2: 0,
                bf_offset_bits: 0,
            }
        }
        pub fn load_from_file() -> BMPFileHeader {
            BMPFileHeader::new()
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

    #[derive(Debug)]
    pub struct RGBTriple {
        rgbt_blue: u8,
        rgbt_green: u8,
        rgbt_red: u8,
    }

    #[derive(Debug)]
    pub struct BMPInfo {
        /// A BITMAPINFOHEADER structure that contains information about the dimensions of color format.
        bmi_header: BMPInfoHeader,
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

    #[derive(Debug)]
    pub struct CIEXYZTriple {
        ciexyz_red: CIEXYZ,
        ciexyz_green: CIEXYZ,
        ciexyz_blue: CIEXYZ,
    }

    type Fxpt2Dot30 = u32;
    #[derive(Debug)]
    pub struct CIEXYZ {
        ciexyz_x: Fxpt2Dot30,
        ciexyz_y: Fxpt2Dot30,
        ciexyz_z: Fxpt2Dot30,
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
}

pub fn print_bmp_stats(f: &bmp::BMPFileHeader) {
    println!("Hello World! and {:?}", f);
}

pub fn main() {
    let fh = bmp::BMPFileHeader::new();
    print_bmp_stats(&fh);
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
