extern crate byteorder;
extern crate clap;

pub mod bmp;
mod args;

pub fn main() {
    let app = args::build_app("bmper");
    if let Some(matches) = app.subcommand_matches("meta") {
        let filename = matches.value_of("FILE").unwrap();
        println!("Info from file {:?}", filename);
        match bmp::BMPImage::load_from_file(filename) {
            Ok(bmp_info) => {
                if matches.is_present("raw") {
                    println!("{:?}\n{:?}", bmp_info.header, bmp_info.info.bmi_header);
                } else {
                    println!("{}", bmp_info);
                }
            },
            Err(e) => println!("{:?}", e),
        }
    }
    if let Some(matches) = app.subcommand_matches("grayscale") {
        let src = matches.value_of("SRC").unwrap();
        let dst = matches.value_of("DST").unwrap();
        let image = bmp::BMPImage::load_meta_and_bitmap(src).unwrap();
        image.grayscale();
        println!("{}\nSRC: {} -> DST {}", image.info, src, dst);
        image.save_to_file(dst).unwrap();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
