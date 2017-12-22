extern crate byteorder;
extern crate rand;
#[macro_use]
extern crate clap;

pub mod bmp;
pub mod encoding;
mod args;

pub fn main() {
    let app = args::build_app("bmper");
    if let Some(matches) = app.subcommand_matches("meta") {
        let filename = matches.value_of("FILE").unwrap();
        println!("Info from file {:?}", filename);
        let bmp_info = bmp::BMPImage::load_from_file(filename)
            .expect(format!("Source file {}", filename).as_ref());
        if matches.is_present("raw") {
            println!("{:?}\n{:?}", bmp_info.header, bmp_info.info.bmi_header);
        } else {
            println!("{}", bmp_info);
        }
        if matches.is_present("colors") {
            println!("{:?}", bmp_info.info.bmi_colors);
        }
    } else if let Some(matches) = app.subcommand_matches("grayscale") {
        let src = matches.value_of("SRC").unwrap();
        let dst = matches.value_of("DST").unwrap();
        let mut image = bmp::BMPImage::load_meta_and_bitmap(src).expect(src);
        image.grayscale();
        image.save_to_file(dst).expect(dst);
    } else if let Some(matches) = app.subcommand_matches("border") {
        let src = matches.value_of("SRC").unwrap();
        let dst = matches.value_of("DST").unwrap();
        let mut width: i16 = 15; // pixels
        if matches.is_present("width") {
            width = value_t_or_exit!(matches, "width", i16);
        }
        let mut image = bmp::BMPImage::load_meta_and_bitmap(src).expect(src);
        image.decode_bitmap();
        image.border(width);
        image.save_to_file(dst).expect(dst);
    } else if let Some(matches) = app.subcommand_matches("decode") {
        let src = matches.value_of("SRC").unwrap();
        let dst = matches.value_of("DST").unwrap();
        let mut image = bmp::BMPImage::load_meta_and_bitmap(src).expect(src);
        image.decode_bitmap();
        image.save_to_file(dst).expect(dst);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
