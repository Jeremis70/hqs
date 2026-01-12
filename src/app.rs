use crate::cli::{CaptureArgs, Cli, Cmd, FileType};
use chrono::Local;
use grim_rs::{Box as GrimBox, CaptureParameters, Grim};
use std::env;
use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

pub fn run(cli: Cli) -> i32 {
    match cli.cmd {
        Cmd::Capture(args) => match run_capture(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("Error: {err}");
                1
            }
        },
    }
}

fn run_capture(args: CaptureArgs) -> grim_rs::Result<()> {
    let output_file = if let Some(ref path) = args.output_file {
        path.clone()
    } else {
        generate_default_filename(args.filetype)?
    };

    let mut grim = Grim::new()?;

    let region: Option<GrimBox> = match args.geometry.as_deref() {
        None => None,
        Some("-") => Some(Grim::read_region_from_stdin()?),
        Some(spec) => Some(spec.parse()?),
    };

    // Match grim: if -s isn't provided, default to the greatest output scale factor among
    // outputs intersecting the capture geometry.
    let scale_region = if let Some(ref output_name) = args.output {
        let outputs = grim.get_outputs()?;
        let output = outputs
            .iter()
            .find(|o| o.name() == output_name)
            .ok_or_else(|| grim_rs::Error::OutputNotFound(output_name.clone()))?;
        Some(*output.geometry())
    } else {
        region
    };

    let default_scale = match args.scale {
        Some(s) => s,
        None => grim.greatest_scale_for_region(scale_region)?,
    };

    let result = if let Some(ref output_name) = args.output {
        if args.cursor {
            let mut params = CaptureParameters::new(output_name.clone()).overlay_cursor(true);
            if let Some(region) = region {
                params = params.region(region);
            }
            if let Some(scale) = args.scale {
                params = params.scale(scale);
            }

            let multi_result = grim.capture_outputs_with_scale(vec![params], default_scale)?;
            if let Some(capture_result) = multi_result.get(output_name) {
                capture_result.clone()
            } else {
                return Err(grim_rs::Error::OutputNotFound(output_name.clone()));
            }
        } else {
            grim.capture_output_with_scale(output_name, default_scale)?
        }
    } else if let Some(region) = region {
        grim.capture_region_with_scale(region, default_scale)?
    } else {
        grim.capture_all_with_scale(default_scale)?
    };

    save_or_write_result(&grim, &result, &output_file, &args)
}

fn save_or_write_result(
    grim: &Grim,
    result: &grim_rs::CaptureResult,
    output_file: &str,
    args: &CaptureArgs,
) -> grim_rs::Result<()> {
    if output_file == "-" {
        write_to_stdout(grim, result, args)
    } else {
        save_to_file(grim, result, output_file, args)
    }
}

fn write_to_stdout(
    grim: &Grim,
    result: &grim_rs::CaptureResult,
    args: &CaptureArgs,
) -> grim_rs::Result<()> {
    match args.filetype {
        FileType::Png => write_png_to_stdout(grim, result, args.level),
        FileType::Ppm => grim.write_ppm_to_stdout(result.data(), result.width(), result.height()),
        FileType::Jpeg => write_jpeg_to_stdout(grim, result, args.quality),
    }
}

fn save_to_file(
    grim: &Grim,
    result: &grim_rs::CaptureResult,
    output_file: &str,
    args: &CaptureArgs,
) -> grim_rs::Result<()> {
    let path = Path::new(output_file);
    match args.filetype {
        FileType::Png => save_png_to_file(grim, result, path, args.level),
        FileType::Ppm => grim.save_ppm(result.data(), result.width(), result.height(), path),
        FileType::Jpeg => save_jpeg_to_file(grim, result, path, args.quality),
    }
}

fn write_png_to_stdout(
    grim: &Grim,
    result: &grim_rs::CaptureResult,
    compression_level: u8,
) -> grim_rs::Result<()> {
    if compression_level == 6 {
        grim.write_png_to_stdout(result.data(), result.width(), result.height())
    } else {
        grim.write_png_to_stdout_with_compression(
            result.data(),
            result.width(),
            result.height(),
            compression_level,
        )
    }
}

fn save_png_to_file(
    grim: &Grim,
    result: &grim_rs::CaptureResult,
    path: &Path,
    compression_level: u8,
) -> grim_rs::Result<()> {
    if compression_level == 6 {
        grim.save_png(result.data(), result.width(), result.height(), path)
    } else {
        grim.save_png_with_compression(
            result.data(),
            result.width(),
            result.height(),
            path,
            compression_level,
        )
    }
}

fn write_jpeg_to_stdout(
    grim: &Grim,
    result: &grim_rs::CaptureResult,
    quality: u8,
) -> grim_rs::Result<()> {
    if quality == 80 {
        grim.write_jpeg_to_stdout(result.data(), result.width(), result.height())
    } else {
        grim.write_jpeg_to_stdout_with_quality(
            result.data(),
            result.width(),
            result.height(),
            quality,
        )
    }
}

fn save_jpeg_to_file(
    grim: &Grim,
    result: &grim_rs::CaptureResult,
    path: &Path,
    quality: u8,
) -> grim_rs::Result<()> {
    if quality == 80 {
        grim.save_jpeg(result.data(), result.width(), result.height(), path)
    } else {
        grim.save_jpeg_with_quality(
            result.data(),
            result.width(),
            result.height(),
            path,
            quality,
        )
    }
}

fn generate_default_filename(filetype: FileType) -> grim_rs::Result<String> {
    // Format: YYYYMMDD_HHhMMmSSs_hqs.ext (e.g., 20241004_10h30m45s_hqs.png)
    let now = Local::now();
    let timestamp = now.format("%Y%m%d_%Hh%Mm%Ss");

    let ext = match filetype {
        FileType::Png => "png",
        FileType::Ppm => "ppm",
        FileType::Jpeg => "jpeg",
    };

    let output_dir = get_output_dir();
    let filename = format!("{}_hqs.{}", timestamp, ext);
    Ok(output_dir.join(filename).to_string_lossy().to_string())
}

/// ~/.config/user-dirs.dirs
fn get_xdg_pictures_dir() -> Option<PathBuf> {
    // XDG_PICTURES_DIR (env)
    if let Ok(pictures_dir) = env::var("XDG_PICTURES_DIR") {
        let expanded = expand_home_dir(&pictures_dir);
        return Some(PathBuf::from(expanded));
    }

    // Parse ~/.config/user-dirs.dirs
    let config_home = env::var("XDG_CONFIG_HOME").ok().or_else(|| {
        env::var("HOME")
            .ok()
            .map(|home| format!("{}/.config", home))
    })?;

    let user_dirs_file = PathBuf::from(config_home).join("user-dirs.dirs");
    if !user_dirs_file.exists() {
        return None;
    }

    let file = fs::File::open(user_dirs_file).ok()?;
    let reader = io::BufReader::new(file);

    for line in reader.lines().map_while(Result::ok) {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with("XDG_PICTURES_DIR=")
            && let Some(value) = line.strip_prefix("XDG_PICTURES_DIR=")
        {
            let value = value.trim_matches('"').trim_matches('\'');
            let expanded = expand_home_dir(value);
            return Some(PathBuf::from(expanded));
        }
    }

    None
}

/// Expand $HOME in paths
fn expand_home_dir(path: &str) -> String {
    if path.starts_with("$HOME")
        && let Ok(home) = env::var("HOME")
    {
        return path.replace("$HOME", &home);
    }
    path.to_string()
}

/// GRIM_DEFAULT_DIR > XDG_PICTURES_DIR > "."
fn get_output_dir() -> PathBuf {
    if let Ok(default_dir) = env::var("GRIM_DEFAULT_DIR") {
        let path = PathBuf::from(default_dir);
        if path.exists() || path.parent().map(|p| p.exists()).unwrap_or(false) {
            return path;
        }
    }

    if let Some(pictures_dir) = get_xdg_pictures_dir()
        && pictures_dir.exists()
    {
        return pictures_dir;
    }

    PathBuf::from(".")
}
