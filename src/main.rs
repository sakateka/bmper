extern crate byteorder;
extern crate clap;

use std::error::Error;
pub mod bmp;
mod args;

pub fn main() {
    let app = args::build_app("bmper");
    if let Some(matches) = app.subcommand_matches("meta") {
        let filename = matches.value_of("FILE").unwrap();
        println!("Info from file {:?}", filename);
        match bmp::BMPFileHeader::load_from_file(filename) {
            Ok(fh) => {
                println!("{:?}", fh);
                let fi = bmp::BMPGenericInfo::load_from_file(filename);
                if fi.is_ok() {
                    println!("{:?}", fi);
                } else {
                    println!("Failed to parse metadata: {}", fi.err().unwrap().description());
                }
            },
            Err(err) => {
                println!("Unsupported file format: {}", err.description());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
