use crate::cli::{CaptureArgs, Cli, Cmd, FileType, FinalizeArgs};
use chrono::Local;
use grim_rs::{Box as GrimBox, CaptureParameters, Grim};
use image::DynamicImage;
use image::GenericImageView;
use std::env;
use std::fmt;
use std::fs;
use std::io::IsTerminal;
use std::io::Write;
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
        Cmd::Finalize(args) => match run_finalize(args) {
            Ok(()) => 0,
            Err(err) => {
                eprintln!("Error: {err}");
                1
            }
        },
    }
}

#[derive(Debug)]
struct FinalizeError(String);

impl fmt::Display for FinalizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for FinalizeError {}

fn run_capture(args: CaptureArgs) -> grim_rs::Result<()> {
    let output_file = if let Some(path) = args.output_file.as_deref() {
        path.to_path_buf()
    } else if !io::stdout().is_terminal() {
        // Allow piping without explicitly passing "-":
        //   hqs capture | wl-copy -t image/png
        PathBuf::from("-")
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
        if let Some(region) = region {
            let mut params =
                CaptureParameters::new(output_name.clone()).overlay_cursor(args.cursor);
            params = params.region(region);

            let multi_result = grim.capture_outputs_with_scale(vec![params], default_scale)?;
            multi_result
                .get(output_name)
                .cloned()
                .ok_or_else(|| grim_rs::Error::OutputNotFound(output_name.clone()))?
        } else {
            // Output complet : utiliser l'API dédiée (fix bug noir + mauvais scale)
            if args.cursor {
                grim.capture_output_with_scale_and_cursor(output_name, default_scale, true)?
            } else {
                grim.capture_output_with_scale(output_name, default_scale)?
            }
        }
    } else if let Some(region) = region {
        if args.cursor {
            grim.capture_region_with_scale_and_cursor(region, default_scale, true)?
        } else {
            grim.capture_region_with_scale(region, default_scale)?
        }
    } else if args.cursor {
        grim.capture_all_with_scale_and_cursor(default_scale, true)?
    } else {
        grim.capture_all_with_scale(default_scale)?
    };

    save_or_write_result(&grim, &result, &output_file, &args)
}

fn run_finalize(args: FinalizeArgs) -> Result<(), FinalizeError> {
    let [x, y, w, h] = parse_crop_px(&args.crop_px)?;

    let base = image::open(&args.base).map_err(|e| {
        FinalizeError(format!(
            "Failed to open base image '{}': {e}",
            args.base.display()
        ))
    })?;

    let cropped = crop_image_px(&base, x, y, w, h)?;

    let output_path = if let Some(path) = args.output_file.as_deref() {
        path.to_path_buf()
    } else {
        generate_default_finalize_filename()
    };

    if output_path == Path::new("-") {
        write_png_to_stdout_image(&cropped)?;
        return Ok(());
    }

    if let Some(parent) = output_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|e| {
            FinalizeError(format!(
                "Failed to create output directory '{}': {e}",
                parent.display()
            ))
        })?;
    }

    save_dynamic_image(&cropped, &output_path).map_err(|e| {
        FinalizeError(format!(
            "Failed to save output '{}': {e}",
            output_path.display()
        ))
    })?;

    Ok(())
}

fn parse_crop_px(values: &[u32]) -> Result<[u32; 4], FinalizeError> {
    if values.len() != 4 {
        return Err(FinalizeError(
            "--crop-px expects exactly 4 integers: x y w h".to_string(),
        ));
    }
    Ok([values[0], values[1], values[2], values[3]])
}

fn crop_image_px(
    image: &DynamicImage,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
) -> Result<DynamicImage, FinalizeError> {
    if w == 0 || h == 0 {
        return Err(FinalizeError("Crop width/height must be > 0".to_string()));
    }

    let (img_w, img_h) = image.dimensions();
    let x2 = x
        .checked_add(w)
        .ok_or_else(|| FinalizeError("Crop overflows".to_string()))?;
    let y2 = y
        .checked_add(h)
        .ok_or_else(|| FinalizeError("Crop overflows".to_string()))?;

    if x >= img_w || y >= img_h || x2 > img_w || y2 > img_h {
        return Err(FinalizeError(format!(
            "Crop is out of bounds: image is {img_w}x{img_h}, crop is x={x} y={y} w={w} h={h}"
        )));
    }

    Ok(image.crop_imm(x, y, w, h))
}

fn generate_finalize_filename(ext: &str) -> String {
    let now = Local::now();
    let timestamp = now.format("%Y%m%d_%Hh%Mm%Ss");
    format!("{}_hqs_final.{ext}", timestamp)
}

fn generate_default_finalize_filename() -> PathBuf {
    // Match capture behavior request: default into current directory.
    PathBuf::from(generate_finalize_filename("png"))
}

fn write_png_to_stdout_image(image: &DynamicImage) -> Result<(), FinalizeError> {
    use std::io::Cursor;

    let mut bytes = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
        .map_err(|e| FinalizeError(format!("Failed to encode PNG: {e}")))?;

    let mut stdout = io::stdout().lock();
    stdout
        .write_all(&bytes)
        .map_err(|e| FinalizeError(format!("Failed to write to stdout: {e}")))?;
    Ok(())
}

fn save_dynamic_image(image: &DynamicImage, path: &Path) -> Result<(), image::ImageError> {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
    {
        Some(ext) if ext == "jpg" || ext == "jpeg" => {
            use image::ColorType;
            use image::codecs::jpeg::JpegEncoder;
            use std::io::BufWriter;

            let file = fs::File::create(path)?;
            let mut writer = BufWriter::new(file);
            let mut encoder = JpegEncoder::new_with_quality(&mut writer, 80);
            let rgb = image.to_rgb8();
            encoder.encode(&rgb, rgb.width(), rgb.height(), ColorType::Rgb8.into())
        }
        Some(ext) if ext == "png" => image.save_with_format(path, image::ImageFormat::Png),
        Some(ext) if ext == "ppm" || ext == "pnm" => {
            image.save_with_format(path, image::ImageFormat::Pnm)
        }
        _ => image.save_with_format(path, image::ImageFormat::Png),
    }
}

fn save_or_write_result(
    grim: &Grim,
    result: &grim_rs::CaptureResult,
    output_file: &Path,
    args: &CaptureArgs,
) -> grim_rs::Result<()> {
    if output_file == Path::new("-") {
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
    output_file: &Path,
    args: &CaptureArgs,
) -> grim_rs::Result<()> {
    match args.filetype {
        FileType::Png => save_png_to_file(grim, result, output_file, args.level),
        FileType::Ppm => grim.save_ppm(result.data(), result.width(), result.height(), output_file),
        FileType::Jpeg => save_jpeg_to_file(grim, result, output_file, args.quality),
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

fn generate_default_filename(filetype: FileType) -> grim_rs::Result<PathBuf> {
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
    Ok(output_dir.join(filename))
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
