//! Native image printing for terminals that expose the Kitty graphics
//! protocol through the environment.
//!
//! `viuer` detects support by writing a DA1 query and reading the terminal's
//! reply from stdin with `console::Term::read_key`, which briefly restores the
//! tty between every byte. A reply that lands in that window is echoed by the
//! line discipline (`^[[?62;52;c`), and the probe can block or swallow typed
//! input. When Kitty is identified up front we can send the graphics escape
//! directly instead: no capability query, no stdin read and no DSR wait.

use base64::Engine as _;
use crossterm::cursor::MoveRight;
use crossterm::execute;
use image::DynamicImage;
use std::io::{self, IsTerminal, Write};

const CHUNK_LEN: usize = 4096;

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
) -> io::Result<(u32, u32)> {
    let (cols, rows) = cells_for(img.width(), img.height(), width, height);
    if x > 0 {
        execute!(out, MoveRight(x))?;
    }
    out.write_all(&kitty_sequences(img, cols, rows))?;
    out.flush()?;
    Ok((cols, rows))
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
        let size = print_kitty(&mut out, &img, 2, None, 7).unwrap();
        assert_eq!(size, (2, 1));
        let out = String::from_utf8(out).unwrap();
        assert!(
            out.starts_with("\x1b[7C\x1b_G"),
            "offset precedes the escape"
        );
    }
}
