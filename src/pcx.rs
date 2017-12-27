//! # Format desciption
//! https://en.wikipedia.org/wiki/PCX#PCX_file_format
//! https://www.fileformat.info/format/pcx/egff.htm

extern crate gdk_pixbuf;

use std::io::{self, BufRead, BufReader, SeekFrom, Seek};
use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use self::gdk_pixbuf::Pixbuf;

use bmp;

#[derive(Debug, Copy, Clone)]
pub struct RGBTriple {
    red: u8,
    green: u8,
    blue: u8,
}

impl RGBTriple {
    pub fn load_from_reader<R: ?Sized + BufRead>(r: &mut R) -> io::Result<RGBTriple> {
        Ok(RGBTriple {
            red: r.read_u8()?,
            green: r.read_u8()?,
            blue: r.read_u8()?,
        })
    }
}

#[derive(Debug)]
pub struct PCXHeader {
    version: u8,
    bitsperpixel: u8,
    colorplanes: u8,
    bytesperline: u16,
    height: i16,
    width: i16,
    palette: Vec<RGBTriple>,
}

impl PCXHeader {
    pub fn load_from_reader<R: ?Sized + BufRead + Seek>(r: &mut R) -> io::Result<PCXHeader> {
        r.seek(SeekFrom::Start(1))?; // manufacturer
        let version = r.read_u8()?;
        r.seek(SeekFrom::Current(1))?; // encoding
        let bitsperpixel = r.read_u8()?;
        let xstart = r.read_i16::<LittleEndian>()?;
        let ystart = r.read_i16::<LittleEndian>()?;
        let xend = r.read_i16::<LittleEndian>()?;
        let yend = r.read_i16::<LittleEndian>()?;
        let height = yend - ystart + 1;
        let width = xend - xstart + 1;
        // skip u16(horizdpi) and u16(vertdpi) + 16 (u24)colors palette + 1(u8) reserved
        r.seek(SeekFrom::Current(53))?;
        let colorplanes = r.read_u8()?;
        let bytesperline = r.read_u16::<LittleEndian>()?;

        let mut bpp = 8;
        if version == 5 && bitsperpixel == 8 && colorplanes == 3 {
            bpp = 24;
        }

        let mut palette = Vec::new();
        if bpp == 8 {
            if colorplanes != 1 || bitsperpixel != 8 {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("PCX bitsperpixel={}/colorplanes={} not supported",
                            bitsperpixel, colorplanes),
                ));
            }
            // https://en.wikipedia.org/wiki/PCX#PCX_file_format
            r.seek(SeekFrom::End(-769))?; // try find 256 color palette
            if r.read_u8()? != 12u8 {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("PCX 8bpp without 256 color palette no supported!"),
                ));
            }
            for _ in 0..256 {
                palette.push(RGBTriple::load_from_reader(r)?);
            }
        }

        Ok(PCXHeader {
            version: version,
            bitsperpixel: bpp,
            colorplanes: colorplanes,
            bytesperline: bytesperline,
            height: height,
            width: width,
            palette: palette,
        })
    }
}

pub fn decode_line<R: ?Sized + BufRead + Seek>(r: &mut R, row_stride: u16) -> io::Result<Vec<u8>> {
    let mut scanline = Vec::with_capacity(row_stride as usize);
    let mut idx = row_stride as i32;
    let counter_marker = 0b1100_0000u8;
    while idx > 0 {
        let byte = r.read_u8()?;
        let (run_count, run_value) = if (byte & counter_marker) == counter_marker {
            (byte & !counter_marker, r.read_u8()?)
        } else {
            (1, byte)
        };
        let current_len = scanline.len();
        idx -= run_count as i32;
        scanline.resize(current_len + run_count as usize, run_value);
    }
    Ok(scanline)
}

pub fn pixbuf_from_file(name: &str) -> io::Result<Pixbuf> {
    let mut f = BufReader::new(File::open(name)?);
    let header = PCXHeader::load_from_reader(&mut f)?;
    f.seek(SeekFrom::Start(128))?; // skip header

    let pcx_row_stride = header.colorplanes as u16 * header.bytesperline;
    let last_row_len = header.width * 3;
    let cap = (header.height - 1) as usize * pcx_row_stride as usize + last_row_len as usize;
    let mut data = Vec::with_capacity(cap);
    for _ in 0..header.height {
        let scanline = decode_line(&mut f, pcx_row_stride)?;
        for pixel_idx in 0..header.width {
            if header.bitsperpixel == 24 {
                data.push(scanline[pixel_idx as usize]);                      // red
                data.push(scanline[(header.width + pixel_idx) as usize]);   // green
                data.push(scanline[(header.width*2 + pixel_idx) as usize]); // blue
            } else {
                let palette_idx = scanline[pixel_idx as usize] as usize;
                let rgb = header.palette[palette_idx];
                data.push(rgb.red);
                data.push(rgb.green);
                data.push(rgb.blue);
            }
        }
    }

    let pixbuf = Pixbuf::new_from_vec(
        data,                                   // vec
        0 as gdk_pixbuf::Colorspace,            // GDK_COLORSPACE_RGB = 0 colorspace
        false,                                  // has_alpha
        8,                                      // bits_per_sample (only 8 bps supported)
        header.width as i32,
        header.height as i32,
        last_row_len as i32,                           // row_stride for pixbuf
    );
    Ok(pixbuf)
}

struct ColorRonder {
    from: u32,
    to: u32,
    color: bmp::RGBQuad,
}

impl ColorRonder {
    fn get_color() -> bmp::RGBQuad {
        bmp::RGBQuad::new()
    }
    fn can_round(src_color: bmp::RGBQuad) -> bool {
        false
    }
}

struct PaletteRounder {
    palette: Vec<ColorRonder>
}

impl PaletteRounder {
    fn from_pxc_palete(palette: Vec<RGBTriple>) -> PaletteRounder {
        PaletteRounder{
            palette: Vec::new(),
        }
    }
}

pub fn pcx_256colors_to_bmp_16colors(src_file: &str, dst_file: &str) -> io::Result<()> {
    let mut src = BufReader::new(File::open(src_file)?);
    let header = PCXHeader::load_from_reader(&mut src)?;
    src.seek(SeekFrom::Start(128))?; // skip header

    // https://en.wikipedia.org/wiki/BMP_file_format
    let bmp_row_stride = (8 * header.width + 31)/32*4;

    let mut dst_bmp = bmp::BMPImage {
        header: bmp::BMPFileHeader::new(
                (  // BMP file size
                    14 + // file header size
                    40 + // bmp info header size
                    4 * 16 + // palette size
                    bmp_row_stride * header.height // bitmap size
                ) as i32,
                14 + 40 + 4*256, // offset to bitmap bits
            ),
        info: bmp::BMPInfo {
            bmi_header: bmp::BMPGenericInfoHeader::Info(bmp::BMPInfoHeader::new(
                header.width as i32,
                header.height as i32,
                8, // bits per pixel
                (bmp_row_stride * header.height) as i32, // bitmap size
                0, 0,  // x, y pixels per meter (ignored)
                16, 16,  // colors used, important
            )),
            bmi_colors: vec![bmp::RGBQuad::new(); 16],
        },
        bitmap: bmp::Bitmap::new(),
    };

    let pcx_row_stride = header.colorplanes as u16 * header.bytesperline;
    for _ in 0..header.height {
        let scanline = decode_line(&mut src, pcx_row_stride)?;
        for pixel_idx in 0..header.width {
            let palette_idx = scanline[pixel_idx as usize] as usize;
            let rgb = header.palette[palette_idx];
        }
    }
    Ok(())
}
