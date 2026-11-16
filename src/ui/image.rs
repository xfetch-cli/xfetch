//! Native image printing for terminals that expose the Kitty graphics
//! protocol through the environment.
//!
//! `viuer` detects support by writing a DA1 query and reading the terminal's
//! reply from stdin with `console::Term::read_key`, which briefly restores the
//! tty between every byte. A reply that lands in that window is echoed by the
//! line discipline (`^[[?62;52;c`), and the probe can block or swallow typed
//! input. When Kitty is identified up front we can send the graphics escape
//! directly instead: no capability query, no stdin read and no DSR wait.
//!
//! Pixels can travel inline (base64 in the graphics escape, the default) or
//! through a per-render temporary file that Kitty reads and deletes. The file
//! transport is opt-in (`logo_kitty_transfer: "file"`) and falls back to
//! inline whenever the file cannot be created or written.

use base64::Engine as _;
use crossterm::cursor::MoveRight;
use crossterm::execute;
use image::DynamicImage;
use std::fs::{self, OpenOptions};
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::sync::Once;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

const CHUNK_LEN: usize = 4096;
const TEMP_PREFIX: &str = "xfetch-tty-graphics-protocol-";
const TEMP_SUFFIX: &str = ".rgba";
const STALE_TEMP_AGE: Duration = Duration::from_secs(60 * 60);
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
static TEMP_PRUNE: Once = Once::new();

/// Transport used to hand the raw RGBA pixels to Kitty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KittyTransfer {
    /// Pixels are base64-encoded inside the graphics escape (default).
    Inline,
    /// Pixels are written to a temp file and only its path is sent.
    File,
}

impl KittyTransfer {
    pub(crate) fn from_config(value: Option<&str>) -> Self {
        match value {
            Some("file") => Self::File,
            _ => Self::Inline,
        }
    }
}

/// Whether the Kitty graphics protocol can be used without probing the
/// terminal: Kitty is running locally, stdout is a terminal and no terminal
/// multiplexer (which would swallow the graphics escape) sits in between.
pub(crate) fn direct_kitty_available() -> bool {
    std::io::stdout().is_terminal()
        && std::env::var_os("KITTY_WINDOW_ID").is_some()
        && std::env::var_os("TMUX").is_none()
        && std::env::var_os("ZELLIJ").is_none()
        && std::env::var_os("STY").is_none()
}

/// Display size in terminal cells, matching `viuer`'s fit logic so layouts
/// keep the same dimensions: width only scales down and the cell aspect ratio
/// is 2:1 unless an explicit height is configured.
pub(crate) fn cells_for(
    image_width: u32,
    image_height: u32,
    width: u32,
    height: Option<u32>,
) -> (u32, u32) {
    if let Some(rows) = height {
        return (width, rows);
    }
    if image_width <= width {
        return (image_width, (image_height / 2 + image_height % 2).max(1));
    }
    let scaled = (u64::from(image_height) * u64::from(width) / u64::from(image_width)) as u32;
    (width, (scaled / 2).max(1))
}

/// Encodes the image as Kitty graphics protocol escapes: raw RGBA chunks of at
/// most [`CHUNK_LEN`] base64 characters, displayed at the current cell.
pub(crate) fn kitty_sequences(img: &DynamicImage, cols: u32, rows: u32) -> Vec<u8> {
    let rgba = img.to_rgba8();
    let encoded = base64::engine::general_purpose::STANDARD.encode(rgba.as_raw());
    let mut chunks = encoded.as_bytes().chunks(CHUNK_LEN).peekable();
    let mut out = Vec::with_capacity(encoded.len() + 128);

    if let Some(first) = chunks.next() {
        let more = u8::from(chunks.peek().is_some());
        let header = format!(
            "\x1b_Gf=32,a=T,t=d,s={},v={},c={},r={},m={};",
            img.width(),
            img.height(),
            cols,
            rows,
            more
        );
        out.extend_from_slice(header.as_bytes());
        out.extend_from_slice(first);
        out.extend_from_slice(b"\x1b\\");
    }

    while let Some(chunk) = chunks.next() {
        let more = u8::from(chunks.peek().is_some());
        out.extend_from_slice(format!("\x1b_Gm={};", more).as_bytes());
        out.extend_from_slice(chunk);
        out.extend_from_slice(b"\x1b\\");
    }

    out
}

/// Prints `img` at the cursor's current line, offset `x` cells to the right.
/// Returns the size in cells that the terminal will render.
pub(crate) fn print_kitty<W: Write>(
    out: &mut W,
    img: &DynamicImage,
    width: u32,
    height: Option<u32>,
    x: u16,
    transfer: KittyTransfer,
) -> io::Result<(u32, u32)> {
    let (cols, rows) = cells_for(img.width(), img.height(), width, height);
    if x > 0 {
        execute!(out, MoveRight(x))?;
    }
    match transfer {
        KittyTransfer::Inline => write_inline(out, img, cols, rows)?,
        KittyTransfer::File => write_file_transfer(out, img, cols, rows)?,
    }
    out.flush()?;
    Ok((cols, rows))
}

fn write_inline<W: Write>(out: &mut W, img: &DynamicImage, cols: u32, rows: u32) -> io::Result<()> {
    out.write_all(&kitty_sequences(img, cols, rows))
}

/// Sends the image path instead of the pixels. Any failure while creating or
/// writing the temp file falls back to the inline transport so the logo still
/// renders; Kitty reads and deletes the file once the escape is received.
fn write_file_transfer<W: Write>(
    out: &mut W,
    img: &DynamicImage,
    cols: u32,
    rows: u32,
) -> io::Result<()> {
    write_file_transfer_in(out, img, cols, rows, &std::env::temp_dir())
}

fn write_file_transfer_in<W: Write>(
    out: &mut W,
    img: &DynamicImage,
    cols: u32,
    rows: u32,
    dir: &Path,
) -> io::Result<()> {
    let rgba = img.to_rgba8();
    match write_temp_rgba_in(dir, rgba.as_raw()) {
        Ok(path) => {
            let result = out.write_all(&kitty_file_sequence(&path, img, cols, rows));
            if result.is_err() {
                let _ = fs::remove_file(&path);
            }
            result
        }
        Err(_) => write_inline(out, img, cols, rows),
    }
}

fn kitty_file_sequence(path: &Path, img: &DynamicImage, cols: u32, rows: u32) -> Vec<u8> {
    let encoded =
        base64::engine::general_purpose::STANDARD.encode(path.to_string_lossy().as_bytes());
    format!(
        "\x1b_Gf=32,a=T,t=t,s={},v={},c={},r={};{}\x1b\\",
        img.width(),
        img.height(),
        cols,
        rows,
        encoded
    )
    .into_bytes()
}

fn write_temp_rgba_in(dir: &Path, raw: &[u8]) -> io::Result<PathBuf> {
    prune_stale_temp_files(dir);
    let mut attempt = 0;
    loop {
        let path = dir.join(temp_file_name());
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&path) {
            Ok(mut file) => {
                let written = file.write_all(raw).and_then(|()| file.flush());
                if let Err(err) = written {
                    drop(file);
                    let _ = fs::remove_file(&path);
                    return Err(err);
                }
                return Ok(path);
            }
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists && attempt < 16 => {
                attempt += 1;
            }
            Err(err) => return Err(err),
        }
    }
}

fn temp_file_name() -> String {
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    format!(
        "{TEMP_PREFIX}{}-{counter}-{nanos}{TEMP_SUFFIX}",
        std::process::id()
    )
}

/// Removes stale files left by a terminal that did not delete them. Only runs
/// once per process, and only touches this prefix in the temp directory.
fn prune_stale_temp_files(dir: &Path) {
    TEMP_PRUNE.call_once(|| {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        let now = SystemTime::now();
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            if !name.starts_with(TEMP_PREFIX) || !name.ends_with(TEMP_SUFFIX) {
                continue;
            }
            let Ok(modified) = entry.metadata().and_then(|meta| meta.modified()) else {
                continue;
            };
            if now
                .duration_since(modified)
                .is_ok_and(|age| age > STALE_TEMP_AGE)
            {
                let _ = fs::remove_file(entry.path());
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, RgbaImage};

    #[test]
    fn cells_match_viuer_downscale_only_behaviour() {
        assert_eq!(cells_for(10, 20, 20, None), (10, 10));
        assert_eq!(cells_for(120, 20, 20, None), (20, 1));
        assert_eq!(cells_for(10, 20, 20, Some(5)), (20, 5));
        assert_eq!(cells_for(120, 20, 20, Some(5)), (20, 5));
        assert_eq!(cells_for(1, 1, 20, None), (1, 1));
    }

    #[test]
    fn sequences_are_chunked_and_never_probe_the_terminal() {
        let img = DynamicImage::ImageRgba8(RgbaImage::new(100, 100));
        let out = kitty_sequences(&img, 40, 20);
        let out = String::from_utf8(out).unwrap();

        assert!(out.starts_with("\x1b_Gf=32,a=T,t=d,s=100,v=100,c=40,r=20,m=1;"));
        assert!(
            out.contains("\x1b_Gm=0;"),
            "last chunk terminates the transfer"
        );
        assert!(!out.contains("\x1b[c"), "no DA1 query");
        assert!(!out.contains("\x1b[5n"), "no DSR query");

        let payloads: Vec<&str> = out
            .split("\x1b\\")
            .filter(|part| part.contains('\x1b'))
            .collect();
        assert!(payloads.len() > 1, "large image is chunked");
        for part in payloads {
            let data = part.rsplit_once(';').unwrap().1;
            assert!(data.len() <= CHUNK_LEN, "chunk over protocol limit");
        }
    }

    #[test]
    fn small_image_single_chunk_is_complete() {
        let img = DynamicImage::ImageRgba8(RgbaImage::new(1, 1));
        let out = String::from_utf8(kitty_sequences(&img, 1, 1)).unwrap();
        assert!(out.contains("m=0;"));
        assert!(!out.contains("\x1b_Gm="), "no continuation chunk");
    }

    #[test]
    fn right_offset_moves_cursor_before_the_image() {
        let img = DynamicImage::ImageRgba8(RgbaImage::new(2, 2));
        let mut out = Vec::new();
        let size = print_kitty(&mut out, &img, 2, None, 7, KittyTransfer::Inline).unwrap();
        assert_eq!(size, (2, 1));
        let out = String::from_utf8(out).unwrap();
        assert!(
            out.starts_with("\x1b[7C\x1b_G"),
            "offset precedes the escape"
        );
    }

    #[test]
    fn transfer_from_config_defaults_to_inline() {
        assert_eq!(KittyTransfer::from_config(None), KittyTransfer::Inline);
        assert_eq!(
            KittyTransfer::from_config(Some("inline")),
            KittyTransfer::Inline
        );
        assert_eq!(
            KittyTransfer::from_config(Some("unknown")),
            KittyTransfer::Inline
        );
        assert_eq!(
            KittyTransfer::from_config(Some("file")),
            KittyTransfer::File
        );
    }

    #[test]
    fn file_transfer_sends_the_encoded_path() {
        let img = DynamicImage::ImageRgba8(RgbaImage::new(2, 2));
        let sequence = String::from_utf8(kitty_file_sequence(
            Path::new("/tmp/xfetch-kitty-test.rgba"),
            &img,
            2,
            1,
        ))
        .unwrap();
        assert!(sequence.starts_with("\x1b_Gf=32,a=T,t=t,s=2,v=2,c=2,r=1;"));
        assert!(sequence.ends_with("\x1b\\"));
        assert!(!sequence.contains("\x1b[c"), "no DA1 query");
        assert!(!sequence.contains("\x1b[5n"), "no DSR query");

        let encoded = sequence
            .strip_prefix("\x1b_Gf=32,a=T,t=t,s=2,v=2,c=2,r=1;")
            .unwrap()
            .strip_suffix("\x1b\\")
            .unwrap();
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .unwrap();
        assert_eq!(decoded, b"/tmp/xfetch-kitty-test.rgba");
    }

    #[test]
    fn file_transfer_falls_back_to_inline_when_temp_dir_fails() {
        let img = DynamicImage::ImageRgba8(RgbaImage::new(2, 2));
        let missing = std::env::temp_dir().join(format!("{}-missing", temp_file_name()));
        let mut out = Vec::new();
        write_file_transfer_in(&mut out, &img, 2, 1, &missing).unwrap();
        let out = String::from_utf8(out).unwrap();
        assert!(
            out.starts_with("\x1b_Gf=32,a=T,t=d,"),
            "inline fallback when the temp file cannot be created"
        );
    }

    #[test]
    fn temp_file_roundtrip_uses_private_permissions() {
        let raw: Vec<u8> = (0..16).collect();
        let path = write_temp_rgba_in(&std::env::temp_dir(), &raw).unwrap();
        assert_eq!(fs::read(&path).unwrap(), raw);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
        fs::remove_file(path).unwrap();
    }
}
