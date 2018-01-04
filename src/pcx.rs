//! # Format desciption
//! https://en.wikipedia.org/wiki/PCX#PCX_file_format
//! https://www.fileformat.info/format/pcx/egff.htm

extern crate gdk_pixbuf;

use std::collections::{HashMap, HashSet};
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



const LUMA: (f64, f64, f64)  =  (0.2126/*R*/, 0.7152/*G*/, 0.0722/*B*/);

#[derive(Debug)]
struct Cube {
    range: ((u32, u32 /*red*/), (u32, u32 /*green*/), (u32, u32 /*blue*/)),
    color: RGBTriple,
    colors: Vec<RGBTriple>,
}

impl Cube {
    fn new() -> Cube {
        Cube{
            range: ((0, 0), (0, 0), (0, 0)),
            color: RGBTriple::new(0, 0, 0),
            colors: Vec::new(),
        }
    }
    fn split(&mut self) -> Cube {
        let mut red = 0_f64;
        let mut green = 0_f64;
        let mut blue = 0_f64;

        for c in &self.colors {
            red += c.red as f64;
            green += c.green as f64;
            blue += c.blue as f64;
        }

        red *= LUMA.0;
        green *= LUMA.1;
        blue *= LUMA.2;
        let mut max = red;
        if red < green {
            max = green;
        } else if red < blue {
            max = blue;
        }

        self.colors.sort_by(|a, b| {
            if max == red {
                (a.red, a.green, a.blue).cmp(&(b.red, b.green, b.blue))
            } else if max == green {
                (a.green, a.red, a.blue).cmp(&(b.green, b.red, b.blue))
            } else {
                (a.blue, a.green, a.red).cmp(&(b.blue, b.green, b.red))
            }
        });
        let split_at = self.colors.len()/2;
        let colors = self.colors.split_off(split_at);
        Cube{
            range: ((0, 0), (0, 0), (0, 0)),
            color: RGBTriple::new(0, 0, 0),
            colors: colors,
        }
    }
    fn set_color(&mut self, freq: &HashMap<RGBTriple, f64>) {
        let (mut red, mut green, mut blue) = (0_f64, 0_f64, 0_f64);
        let total = self.colors.iter().fold(0f64, |acc, x| acc + freq.get(x).unwrap());
        for color in &self.colors {
            red += color.red as f64 * freq.get(&color).unwrap();
            green += color.green as f64 * freq.get(&color).unwrap();
            blue += color.blue as f64 * freq.get(&color).unwrap();
        }
        red /= total;
        green /= total;
        blue /= total;
        self.color = RGBTriple::new(red.round() as u8, green.round() as u8, blue.round() as u8);
        println!("Cube color : {:?}", self.color);
    }
}

#[derive(Debug)]
struct Palette {
    bit_count: usize,
    cubes: Vec<Cube>,
    palette: Vec<RGBTriple>,
    frequency: HashMap<RGBTriple, f64>,
}

impl Palette {
    fn from_pcx_palette(palette: &Vec<RGBTriple>, bit_count: usize) -> Palette {
        Palette{
            bit_count: bit_count,
            cubes: Vec::with_capacity(bit_count),
            palette: palette.clone(),
            frequency: HashMap::with_capacity(palette.len()),
        }
    }

    fn compute_frequency(&mut self, bitmap: &Vec<u8>, row_stride: usize, h: &PCXHeader) {
        let mut counter = vec![0f64; h.palette.len()];
        for line_idx in 0..(h.height as usize) {
            for pixel_idx in 0..(h.width as usize) {
                counter[bitmap[row_stride * line_idx + pixel_idx] as usize] += 1f64;
            }
        }
        for idx in 0..counter.len() {
            let prev = match self.frequency.get(&h.palette[idx]) {
                Some(&x) => x,
                None => 0f64,
            };
            self.frequency.insert(h.palette[idx], counter[idx] + prev);
        }
    }

    fn color_delta(a: &RGBTriple, b: &RGBTriple) -> usize {
        ((b.red as i16   - a.red as i16).abs() as usize).pow(2) +
        ((b.green as i16 - a.green as i16).abs() as usize).pow(2) +
        ((b.blue as i16  - a.blue as i16).abs() as usize).pow(2)
    }

    fn round_pcx_palette(&mut self) {
        let mut unique_colors = HashSet::new();
        for c in &self.palette {
            unique_colors.insert(c);
        }
        let mut cube = Cube::new();
        cube.colors.append(&mut unique_colors.iter().map(|c|{*(*c)}).collect());
        self.cubes.push(cube);

        for step in 1..(self.bit_count+1) {
            let mut new_cubes = Vec::with_capacity(step);
            for c in &mut self.cubes {
                new_cubes.push(c.split());
            }
            self.cubes.append(&mut new_cubes);
        }
        for c in &mut self.cubes {
            c.set_color(&self.frequency);
        }
    }

    fn cube_color(&self, c: RGBTriple) -> RGBTriple {
        let mut nearest_cube = 0;
        let mut delta = Palette::color_delta(&c, &self.cubes[0].color);

        for idx in 1..self.cubes.len() {
            let next_delta = Palette::color_delta(&c, &self.cubes[idx].color);
            if delta > next_delta {
                delta = next_delta;
                nearest_cube = idx;
            }
        }
        self.cubes[nearest_cube].color
    }
}

pub fn pcx_256colors_to_bmp_16colors(src_file: &str) -> io::Result<bmp::BMPImage> {
    let mut src = BufReader::new(File::open(src_file)?);
    let header = PCXHeader::load_from_reader(&mut src)?;
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
            bmi_colors: Vec::with_capacity(header.palette.len()),
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

    let mut palette = Palette::from_pcx_palette(&header.palette, 4);
    palette.compute_frequency(&dst_bmp.bitmap.data, bmp_row_stride, &header);
    palette.round_pcx_palette();

    for idx in 0..header.palette.len() {
        dst_bmp.info.bmi_colors.push({
            let c = palette.cube_color(header.palette[idx]);
            bmp::RGBQuad::new(c.red, c.green, c.blue)
        });
    }
    Ok(dst_bmp)
}
