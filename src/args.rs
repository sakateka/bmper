use clap::{App, Arg, ArgMatches, SubCommand};

pub fn build_app<'a>(name: &str) -> ArgMatches<'a> {
    App::new(name)
        .version("0.1.0")
        .author("Sergey K. <s.kacheev@gmail.com>")
        .about("BMP image metadata parser")
        .subcommand(
            SubCommand::with_name("meta")
                .about("Show image metadata")
                .arg(Arg::with_name("raw").long("raw").help("print raw metadata"))
                .arg(
                    Arg::with_name("colors")
                        .short("c")
                        .long("colors")
                        .help("print colors table"),
                )
                .arg(
                    Arg::with_name("FILE")
                        .help("Image file to parse")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(
            SubCommand::with_name("grayscale")
                .about("Grayscale BMP image with palette")
                .arg(
                    Arg::with_name("SRC")
                        .help("Source image file")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::with_name("DST")
                        .help("Destination image file")
                        .required(true)
                        .index(2),
                ),
        )
        .subcommand(
            SubCommand::with_name("border")
                .about("Add a border of random pixels")
                .arg(
                    Arg::with_name("width")
                        .short("w")
                        .long("width")
                        .help("border width (pixels)"),
                )
                .arg(
                    Arg::with_name("SRC")
                        .help("Source image file")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::with_name("DST")
                        .help("Destination image file")
                        .required(true)
                        .index(2),
                ),
        )
        .get_matches()
}
