extern crate gdk_pixbuf;

//! https://www.fileformat.info/format/pcx/egff.htm

use std::io::{self, BufRead, BufReader, SeekFrom, Seek};
use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use self::gdk_pixbuf::Pixbuf;

#[derive(Debug, Copy, Clone)]
pub struct RGBTriple {
    rgb_blue: u8,
    rgb_green: u8,
    rgb_red: u8,
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
        let bpp = r.read_u8()?;
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

        if colorplanes == 1 {
            r.seek(SeekFrom::End(0))?; // try find 256 color palette
        }

        Ok(PCXHeader {
            version: version,
            bitsperpixel: bpp,
            colorplanes: colorplanes,
            bytesperline: bytesperline,
            height: height,
            width: width,
        })
    }
}

pub fn pixbuf_from_file(name: &str) -> io::Result<Pixbuf> {
    let mut f = BufReader::new(File::open(name)?);
    let header = PCXHeader::load_from_reader(&mut f)?;
    let mut bpp = 8;
    if header.version == 5 && header.bitsperpixel == 8 && hader.colorplanes == 3 {
        bpp = 24;
    }

    if bpp == 8 && header.colorplanes != 1) {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("PCX bitsperpixel={}/colorplanes={} not supported",
                    header.bitsperpixel, header.colorplanes),
        ))
    }

    let capacity = header.colorplanes as u16 * header.bytesperline;
    let mut scanline = Vec::with_capacity(capacity as usize);

    Err(io::Error::new(
        io::ErrorKind::Other,
        format!("Failed to load image data: '{}':{}", "Not implemented for", name),
    ))
}
