use clap::{App, Arg, ArgMatches, SubCommand};

pub fn build_app<'a>(name: &str) -> ArgMatches<'a> {
    App::new(name)
        .version("0.1.0")
        .author("Sergey K. <uo0@ya.ru>")
        .about("BMP image metadata parser")
        .subcommand(SubCommand::with_name("meta")
                .about("Show image metadata")
                .arg(Arg::with_name("raw").long("raw").help("print raw metadata"))
                .arg(Arg::with_name("colors")
                        .short("c")
                        .long("colors")
                        .help("print colors table"),
                )
                .arg(Arg::with_name("FILE")
                        .help("Image file to parse")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(SubCommand::with_name("grayscale")
                .about("Grayscale BMP image with palette")
                .arg(Arg::with_name("SRC")
                        .help("Source image file")
                        .required(true)
                        .index(1),
                )
                .arg(Arg::with_name("DST")
                        .help("Destination image file")
                        .required(true)
                        .index(2),
                ),
        )
        .subcommand(SubCommand::with_name("border")
                .about("Add a border of random pixels")
                .arg(Arg::with_name("width")
                        .help("border width (upto 32767 pixels)")
                        .short("w")
                        .long("width")
                        .takes_value(true),
                )
                .arg(Arg::with_name("SRC")
                        .help("Source image file")
                        .required(true)
                        .index(1),
                )
                .arg(Arg::with_name("DST")
                        .help("Destination image file")
                        .required(true)
                        .index(2),
                ),
        )
        .subcommand(SubCommand::with_name("decode")
                .about("Decode encoded bitmap")
                .arg(Arg::with_name("SRC")
                        .help("Source image file")
                        .required(true)
                        .index(1),
                )
                .arg(Arg::with_name("DST")
                        .help("Destination image file")
                        .required(true)
                        .index(2),
                ),
        )
        .subcommand(SubCommand::with_name("convert")
                .about("Convert 256 color PCX to 16 color BMP")
                .arg(Arg::with_name("SRC")
                        .help("Source image file")
                        .required(true)
                        .index(1),
                )
                .arg(Arg::with_name("DST")
                        .help("Destination image file")
                        .required(true)
                        .index(2),
                ),
        )
        .subcommand(SubCommand::with_name("logo")
                .about("Add logo to 24 bit per pixel BMP file")
                .arg(Arg::with_name("SRC")
                        .help("Source image file")
                        .required(true)
                        .index(1),
                )
                .arg(Arg::with_name("DST")
                        .help("Destination image file")
                        .required(true)
                        .index(2),
                )
                .arg(Arg::with_name("LOGO")
                        .help("24 bit per pixel logo BMP file")
                        .required(true)
                        .index(3),
                ),
        )
        .subcommand(SubCommand::with_name("display")
                .about("Display image")
                .arg(Arg::with_name("IMAGE")
                        .help("Image file for displaying")
                        .required(true)
                        .index(1),
                ),
        )
        .get_matches()
}
