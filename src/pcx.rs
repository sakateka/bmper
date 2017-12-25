extern crate gdk_pixbuf;

use std::fmt;
use std::io::{self, BufRead, BufReader, Read, SeekFrom};
use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use self::gdk_pixbuf::Pixbuf;

#[derive(Debug)]
pub struct PCXHeader {
    version: u8,
	bitsperpixel: u8,
	colorplanes: u8,
	bytesperline: u16,
	height: i16,
	width: i16,
	palette: Vec<u8>,
}

impl PCXHeader {
    pub fn load_from_reader<R: ?Sized + BufRead>(r: &mut R) -> io::Result<PCXHeader> {
        r.seek(SeekFrom::Start(1))?; // manufacturer
        let version = r.read_u8::<LittleEndian>()?;
        r.seek(SeekFrom::Current(1))?; // encoding
        let bpp = r.read_u8::<LittleEndian>()?;
        let xmin = r.read_i16::<LittleEndian>()?;
        let ymin = r.read_i16::<LittleEndian>()?;
        let xmax = r.read_i16::<LittleEndian>()?;
        let ymax = r.read_i16::<LittleEndian>()?;
        let height = ymax - ymin;
        let width = xmax - xmin;
        r.seek(SeekFrom::Current(4))?; // horizdpi and vertdpi
        let mut pallette = [0u8;48];
        r.read_exact(&mut pallette)?;
        r.seek(SeekFrom::Current(1))?; // reserved
        let colorplanes = r.read_u8::<LittleEndian>()?;
        let bytesperline = r.read_u8::<LittleEndian>()?;

        r.seek(SeekFrom::Start(128))?; // skip all header

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

pub fn pixbuf_from_file(name: &str) -> io::Result<Pixbuf> {
    let mut f = BufReader::new(File::open(name)?);
    Err(io::Error::new(
        io::ErrorKind::Other,
        format!("Failed to load image data: '{}':{}", "Not implemented for", name),
    ))
}
