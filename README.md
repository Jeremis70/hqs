# hqs

[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE-MIT)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE-APACHE)

Minimal screenshot and image-finalization CLI for Wayland.

`hqs` is designed to be used as a *backend* for a screenshot utility (GUI, launcher script, keybind, etc.):
- it can write images to stdout for easy piping
- it can read a selection geometry from stdin
- it keeps the CLI surface small and predictable

This project provides:
- `hqs capture`: grim-like screenshot capture (via my fork of `grim-rs`)
- `hqs finalize`: crop an existing image by pixel coordinates, and save it (or write to stdout)
- `hqs copy-file`: copy any file to the Wayland clipboard by spawning `wl-copy`

## Requirements

- A Wayland session
- A compositor supporting the protocols needed by `grim-rs` for capture
- For `copy-file` (and for piping to clipboard): `wl-copy` from `wl-clipboard`

Optional (recommended for region selection):
- `slurp` or `quickshell` see hyprquickshot

## Build

```bash
cargo build --release
```

Binary will be at `target/release/hqs`.

## Usage

### Capture

Capture a screenshot.

```bash
hqs capture [OPTIONS] [output-file]
```

Notes:
- If `output-file` is `-`, the image is written to stdout.
- If `output-file` is omitted, a timestamped filename is created.
- If stdout is **not** a TTY and no `output-file` is provided, `hqs capture` automatically writes to stdout (so `hqs capture | ...` works).

Using `slurp` for region selection:
- `slurp` prints a geometry string, which `hqs capture -g` can consume.
- You can either pass it as an argument, or pipe it to `hqs` and use `-g -`.

Examples:

```bash
# Save to a default timestamped file
hqs capture

# Save to a specific file
hqs capture out.png

# Pipe directly to clipboard (Wayland)
hqs capture | wl-copy -t image/png

# Force stdout explicitly
hqs capture - | wl-copy -t image/png

# Select a region with slurp, then capture it
hqs capture -g "$(slurp)" out.png

# Same thing, but via stdin (matches grim-style -g - behavior)
slurp | hqs capture -g - out.png

# Region selection + pipe directly to clipboard
slurp | hqs capture -g - - | wl-copy -t image/png
```

### Finalize (crop)

Crop an existing image using pixel coordinates `(x, y, w, h)`.

```bash
hqs finalize --base <path> --crop-px <x> <y> <w> <h> [output-file]
```

Notes:
- If `output-file` is `-`, the cropped image is written to stdout as PNG.
- If `output-file` is omitted, the result is saved to `./` with a timestamped filename.
- `--delete-base` deletes the base file after a successful finalize (refuses to delete if output path equals base path).

Examples:

```bash
# Save next to current directory with a timestamped name
hqs finalize --base shot.png --crop-px 10 10 800 600

# Save to a chosen path
hqs finalize --base shot.png --crop-px 10 10 800 600 out.png

# Pipe to wl-copy (no intermediate file)
hqs finalize --base shot.png --crop-px 10 10 800 600 - | wl-copy -t image/png

# Crop and delete the base file after success
hqs finalize --base /tmp/shot.png --crop-px 10 10 800 600 --delete-base
```

### Copy file (Wayland clipboard)

Copy a file to the Wayland clipboard by spawning `wl-copy` and writing the file bytes to its stdin.

```bash
hqs copy-file --type <mime> <path>
```

Example:

```bash
hqs copy-file --type image/png ./out.png
```

If `wl-copy` is not installed or not in `PATH`, this command will fail.

## Environment

`capture` default output directory follows the same behavior as `grim`:
- `GRIM_DEFAULT_DIR` (if set and valid)
- otherwise `XDG_PICTURES_DIR` (from `~/.config/user-dirs.dirs`, if available)
- otherwise `.`

`finalize` default output is always `.` (current directory).

## License

Dual-licensed under either:
- Apache License, Version 2.0 (see LICENSE-APACHE)
- MIT license (see LICENSE-MIT)
