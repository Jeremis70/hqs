use clap::{Args, Parser, Subcommand, ValueEnum};
use std::fmt;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "hqs")]
#[command(about = "Minimal CLI", long_about = None, term_width = 100)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Subcommand, Debug)]
pub enum Cmd {
    #[command(
        about = "Capture a screenshot (grim-like)",
        after_help = "If output-file is '-', output to standard output.\nIf no output-file is specified, use a default timestamped filename."
    )]
    Capture(CaptureArgs),

    #[command(
        about = "Finalize an existing image (crop, output)",
        after_help = "If output-file is '-', output to standard output.\nIf no output-file is specified, use a default timestamped filename.\n\nExamples:\n  hqs finalize --base shot.png --crop-px 0 0 200 200 out.png\n  hqs finalize --base shot.png --crop-px 10 10 800 600 - | wl-copy -t image/png"
    )]
    Finalize(FinalizeArgs),

    #[command(
        about = "Copy a file to the Wayland clipboard via wl-copy",
        after_help = "Example:\n  hqs copy-file --type image/png ./image.png"
    )]
    CopyFile(CopyFileArgs),
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum FileType {
    Png,
    Ppm,
    #[value(alias = "jpg")]
    Jpeg,
}

impl fmt::Display for FileType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            FileType::Png => "png",
            FileType::Ppm => "ppm",
            FileType::Jpeg => "jpeg",
        };
        f.write_str(s)
    }
}

#[derive(Args, Debug)]
pub struct CaptureArgs {
    #[arg(
        short = 's',
        value_name = "factor",
        help = "Set the output image's scale factor. Defaults to the greatest output scale factor."
    )]
    pub scale: Option<f64>,

    #[arg(
        short = 'g',
        value_name = "geometry",
        help = "Set the region to capture."
    )]
    pub geometry: Option<String>,

    #[arg(
        short = 't',
        value_name = "png|ppm|jpeg|jpg",
        default_value_t = FileType::Png,
        hide_possible_values = true,
        help = "Set the output filetype."
    )]
    pub filetype: FileType,

    #[arg(
        short = 'q',
        value_name = "quality",
        value_parser = clap::value_parser!(u8).range(0..=100),
        default_value_t = 80,
        help = "Set the JPEG filetype compression rate (0-100)."
    )]
    pub quality: u8,

    #[arg(
        short = 'l',
        value_name = "level",
        value_parser = clap::value_parser!(u8).range(0..=9),
        default_value_t = 6,
        help = "Set the PNG filetype compression level (0-9)."
    )]
    pub level: u8,

    #[arg(
        short = 'o',
        value_name = "output",
        help = "Set the output name to capture."
    )]
    pub output: Option<String>,

    #[arg(short = 'c', help = "Include cursors in the screenshot.")]
    pub cursor: bool,

    #[arg(value_name = "output-file")]
    pub output_file: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct FinalizeArgs {
    #[arg(long, value_name = "path", help = "Base image to finalize.")]
    pub base: PathBuf,

    #[arg(
        long = "crop-px",
        value_names = ["x", "y", "w", "h"],
        num_args = 4,
        required = true,
        help = "Crop rectangle in pixels."
    )]
    pub crop_px: Vec<u32>,

    #[arg(long, help = "Delete the base file after a successful finalize.")]
    pub delete_base: bool,

    #[arg(value_name = "output-file")]
    pub output_file: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct CopyFileArgs {
    #[arg(
        long = "type",
        value_name = "mime",
        help = "MIME type passed to wl-copy (-t)."
    )]
    pub mime_type: String,

    #[arg(value_name = "path", help = "File to copy.")]
    pub path: PathBuf,
}
