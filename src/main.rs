extern crate byteorder;
extern crate clap;

pub mod bmp;
mod args;

pub fn main() {
    let app = args::build_app("bmper");
    if let Some(matches) = app.subcommand_matches("meta") {
        let filename = matches.value_of("FILE").unwrap();
        println!("Info from file {:?}", filename);
        let fh = bmp::BMPFileHeader::load_from_file(filename);
        println!("{:?}", fh);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
