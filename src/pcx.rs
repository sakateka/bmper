extern crate gdk_pixbuf;

use std::io;
use self::gdk_pixbuf::Pixbuf;

pub fn pixbuf_from_file(name: &str) -> Result<Pixbuf, io::Error> {
    Err(io::Error::new(
        io::ErrorKind::Other,
        format!("Failed to load image data: '{}':{}", "Not implemented for", name),
    ))
}
