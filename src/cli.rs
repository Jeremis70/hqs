use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "hqs")]
#[command(about = "Minimal CLI", long_about = None, term_width = 100)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Subcommand, Debug)]
pub enum Cmd {
    #[command(about = "Capture a screenshot (grim-like)")]
    Capture(CaptureArgs),
}

#[derive(Args, Debug)]
pub struct CaptureArgs {
    #[arg(
        short = 's',
        long,
        value_name = "factor",
        help = "Set the output image scale factor (default: max)."
    )]
    pub scale: Option<f32>,

    #[arg(
        short = 'g',
        long,
        value_name = "geometry",
        help = "Set the region to capture."
    )]
    pub geometry: Option<String>,

    #[arg(
        short = 't',
        long,
        value_name = "png|ppm|jpeg",
        value_parser = ["png", "ppm", "jpeg"],
        hide_possible_values = true,
        help = "Set the output filetype. Defaults to png."
    )]
    pub filetype: Option<String>,

    #[arg(
        short = 'q',
        long,
        value_name = "quality",
        value_parser = clap::value_parser!(u8).range(0..=100),
        help = "Set the JPEG filetype quality 0-100. Defaults to 80."
    )]
    pub quality: Option<u8>,

    #[arg(
        short = 'l',
        long,
        value_name = "level",
        value_parser = clap::value_parser!(u8).range(0..=9),
        help = "Set the PNG filetype compression level 0-9. Defaults to 6."
    )]
    pub level: Option<u8>,

    #[arg(
        short = 'o',
        long,
        value_name = "output",
        help = "Set the output name to capture."
    )]
    pub output: Option<String>,

    #[arg(
        short = 'T',
        long,
        value_name = "identifier",
        help = "Set the identifier of a foreign toplevel handle to capture."
    )]
    pub toplevel: Option<String>,

    #[arg(short = 'c', long, help = "Include cursors in the screenshot.")]
    pub cursor: bool,
}
