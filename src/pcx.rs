//! # Format desciption
//! https://en.wikipedia.org/wiki/PCX#PCX_file_format
//! https://www.fileformat.info/format/pcx/egff.htm

extern crate gdk_pixbuf;

use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, SeekFrom, Seek};
use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use self::gdk_pixbuf::Pixbuf;

use bmp;

#[derive(Debug, Copy, Clone, Hash, PartialEq)]
pub struct RGBTriple {
    red: u8,
    green: u8,
    blue: u8,
}

impl ::std::cmp::Eq for RGBTriple {}

impl RGBTriple {
    pub fn new(red: u8, green: u8, blue: u8) -> RGBTriple {
        RGBTriple {
            red: red,
            green: green,
            blue: blue,
        }
    }
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


#[derive(Debug)]
struct PaletteRounder {
    target_size: usize,
    map: HashMap<usize, usize>,
    index: HashMap<RGBTriple, usize>,
    frequency: Vec<usize>,
    palette: Vec<RGBTriple>,
}

impl PaletteRounder {
    fn from_pcx_palette(palette: &Vec<RGBTriple>, target_size: usize) -> PaletteRounder {
        let capacity = palette.len();
        PaletteRounder{
            target_size: target_size,
            map: HashMap::with_capacity(capacity),
            index: HashMap::with_capacity(capacity),
            frequency: vec![0_usize; capacity],
            palette: palette.clone(),
        }
    }

    fn compute_frequency(&mut self, bitmap: &Vec<u8>, row_stride: usize, height: usize, width: usize) {
        for line_idx in 0..height {
            for pixel_idx in 0..width {
                self.frequency[bitmap[row_stride * line_idx + pixel_idx] as usize] += 1;
            }
        }
    }

    fn round_pcx_palette(&mut self) -> Vec<RGBTriple> {
        let palette_len = self.palette.len();
        assert!(self.frequency.len() == palette_len, "Frequency counter size not match palette size");
        let mut color_deltas = Vec::with_capacity(palette_len.pow(2)/2);
        for ic in 0..palette_len {
            for icc in 0..palette_len {
                if ic == icc {
                    continue;
                }
                let c = &self.palette[ic];
                let cc = &self.palette[icc];
                let cur_delta = ((cc.red as i16 - c.red as i16).abs() as usize).pow(2) +
                                ((cc.green as i16 - c.green as i16).abs() as usize).pow(2) +
                                ((cc.blue as i16 - c.blue as i16).abs() as usize).pow(2);
                color_deltas.push((ic, icc, cur_delta));
            }
        }
        color_deltas.sort_by(|a, b| a.2.cmp(&b.2));
        let mut current_size = palette_len;
        'deltas: for d in color_deltas {
            let mut src = d.0;
            let mut dst = d.1;
            if self.frequency[src] < self.frequency[dst] {
                src = d.1;
                dst = d.0;
            }
            let mut from = src;
            let to = dst;
            while let Some(color_idx) = self.map.get(&from) {
                if src == *color_idx || dst == *color_idx {
                    continue 'deltas;
                };
                from = *color_idx;
            }
            if from == to {
                continue 'deltas;
            }
            match self.map.get(&to) {
                None => {
                    self.map.insert(from, to);
                    current_size -= 1;
                    if current_size == self.target_size {
                        break;
                    };
                },
                Some(_) => (),
            }

        }
        for idx in 0..palette_len {
            let mut src = idx;
            let mut from_colors = Vec::new();
            while let Some(color_idx) = self.map.get(&src) {
                from_colors.push(*color_idx);
                src = *color_idx;
            }
            if let Some(to) = from_colors.pop() {
                for from in from_colors {
                    self.palette[from] = self.palette[to];
                }
                self.palette[idx] = self.palette[to];
            }
        }
        for c in &self.palette {
            println!("{:?}", c);
        }
        self.palette.clone()
    }
}

pub fn pcx_256colors_to_bmp_16colors(src_file: &str) -> io::Result<bmp::BMPImage> {
    let mut src = BufReader::new(File::open(src_file)?);
    let header = PCXHeader::load_from_reader(&mut src)?;
    let mut palette_rounder = PaletteRounder::from_pcx_palette(&header.palette, 16);
    src.seek(SeekFrom::Start(128))?; // skip header

    // https://en.wikipedia.org/wiki/BMP_file_format
    let bmp_row_stride = ((8 * header.width + 31)/32*4) as usize;

    let file_header_size = 14 /*header size*/ + 40 /*info header size*/ + 4*256 /*palette size*/;
    let bitmap_size = bmp_row_stride as i32 * header.height as i32;

    let mut dst_bmp = bmp::BMPImage {
        header: bmp::BMPFileHeader::new(file_header_size + bitmap_size, file_header_size),
        info: bmp::BMPInfo {
            bmi_header: bmp::BMPGenericInfoHeader::Info(bmp::BMPInfoHeader::new(
                header.width as i32,
                header.height as i32,
                8, // bits per pixel
                bitmap_size,
                0, 0,  // x, y pixels per meter (ignored)
                256, 16,  // colors used, important
            )),
            bmi_colors: vec![bmp::RGBQuad::new(0, 0, 0); 256],
        },
        bitmap: bmp::Bitmap::with_capacity(bmp_row_stride * header.height as usize),
    };
    let pcx_row_stride = header.colorplanes as u16 * header.bytesperline;
    for _ in 0..(header.height as usize) {
        let mut scanline = decode_line(&mut src, pcx_row_stride)?;
        scanline.resize(bmp_row_stride, 0u8);
        scanline.reverse();
        dst_bmp.bitmap.data.append(&mut scanline);
    }
    dst_bmp.bitmap.data.reverse();
    palette_rounder.compute_frequency(
        &dst_bmp.bitmap.data, bmp_row_stride, header.height as usize, header.width as usize,
    );
    let new_palette: Vec<bmp::RGBQuad> = palette_rounder.round_pcx_palette().iter().map(|c| {
        bmp::RGBQuad::new(c.red, c.green, c.blue)
    }).collect();

    for idx in 0..new_palette.len() {
        dst_bmp.info.bmi_colors[idx] = new_palette[idx];
    }
    Ok(dst_bmp)
}
