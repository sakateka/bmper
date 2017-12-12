use clap::{App, Arg, ArgMatches, SubCommand};

pub fn build_app<'a>(name: &str) -> ArgMatches<'a> {
    App::new(name)
        .version("0.1.0")
        .author("Sergey K. <s.kacheev@gmail.com>")
        .about("BMP image metadata parser")
        .subcommand(
            SubCommand::with_name("meta")
                .about("Show image metadata")
                .arg(
                    Arg::with_name("FILE")
                        .help("Image file to parse")
                        .required(true)
                        .index(1),
                ),
        )
        .get_matches()
}
