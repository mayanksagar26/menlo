//! The profile picture.
//!
//! A preset is just an id in the settings — the picture itself is a file the window
//! already has. An uploaded one is a square JPEG the window has already downscaled,
//! arriving as a `data:` URL and stored beside the config rather than inside it, so
//! `config.json` stays something a person can read.
//!
//! Base64 is decoded here rather than by a crate: it is twenty lines, it keeps the
//! dependency tree where CONTRIBUTING asks for it, and it is tested below.

use crate::config::app_dir;
use crate::error::{Error, Result};
use std::path::PathBuf;

/// The id stored in settings when the picture is one the user supplied.
pub const CUSTOM: &str = "custom";

/// The picture a new profile starts with, and the one a cleared upload falls back to.
/// Matches `DEFAULT_AVATAR` in `src/lib/avatars.ts`.
pub const DEFAULT: &str = "menlo";

/// What an uploaded picture may weigh once decoded. The window sends a 256px JPEG,
/// which is tens of kilobytes; this is a sanity bound, not a target.
const MAX_BYTES: usize = 4 * 1024 * 1024;

pub fn path() -> Result<PathBuf> {
    Ok(app_dir()?.join("avatar.jpg"))
}

/// Store a `data:image/…;base64,…` URL as the profile picture.
pub fn save(data_url: &str) -> Result<()> {
    let bytes = decode_data_url(data_url)?;
    if bytes.len() > MAX_BYTES {
        return Err(Error::config("that picture is too large"));
    }
    if !looks_like_image(&bytes) {
        return Err(Error::config("that file is not a JPEG or PNG"));
    }
    let path = path()?;
    crate::config::write_atomic(&path, &bytes)
}

/// The stored picture as a `data:` URL, or `None` when there is not one.
pub fn load() -> Result<Option<String>> {
    let path = path()?;
    let Ok(bytes) = std::fs::read(&path) else {
        return Ok(None);
    };
    let mime = if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        "image/png"
    } else {
        "image/jpeg"
    };
    Ok(Some(format!("data:{mime};base64,{}", encode(&bytes))))
}

pub fn remove() -> Result<()> {
    let _ = std::fs::remove_file(path()?);
    Ok(())
}

/// JPEG starts `FF D8 FF`; PNG starts with its 8-byte signature. Anything else is
/// refused rather than written to disk and handed back to a webview later.
fn looks_like_image(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xFF, 0xD8, 0xFF])
        || bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A])
}

fn decode_data_url(url: &str) -> Result<Vec<u8>> {
    let (head, payload) = url
        .split_once(',')
        .ok_or_else(|| Error::config("that is not an image the window could read"))?;
    if !head.starts_with("data:") || !head.contains(";base64") {
        return Err(Error::config("that is not an image the window could read"));
    }
    decode(payload)
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[(n >> (18 - i * 6) & 0x3F) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn decode(text: &str) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    for c in text.bytes() {
        if c == b'=' || c.is_ascii_whitespace() {
            continue;
        }
        let Some(v) = ALPHABET.iter().position(|a| *a == c) else {
            return Err(Error::config("that picture could not be read"));
        };
        acc = acc << 6 | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits & 0xFF) as u8);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trips_including_the_padded_tails() {
        for case in [
            &b""[..],
            b"f",
            b"fo",
            b"foo",
            b"foob",
            b"fooba",
            b"foobar",
            &[0xFF, 0xD8, 0xFF, 0x00, 0x10, 0x7F],
        ] {
            assert_eq!(decode(&encode(case)).unwrap(), case, "{case:?}");
        }
        // Known vectors, so a bug in both directions cannot hide.
        assert_eq!(encode(b"foobar"), "Zm9vYmFy");
        assert_eq!(encode(b"fo"), "Zm8=");
        assert_eq!(decode("Zm9vYmFy").unwrap(), b"foobar");
    }

    #[test]
    fn only_an_image_is_accepted() {
        assert!(looks_like_image(&[0xFF, 0xD8, 0xFF, 0xE0]));
        assert!(looks_like_image(&[
            0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A
        ]));
        assert!(!looks_like_image(b"<?php echo 1;"));
        assert!(!looks_like_image(b"GIF89a"));
    }

    #[test]
    fn a_data_url_must_actually_be_one() {
        assert!(decode_data_url("data:image/jpeg;base64,Zm9vYmFy").is_ok());
        assert!(decode_data_url("https://example.test/a.jpg").is_err());
        assert!(decode_data_url("data:image/jpeg,notbase64").is_err());
        assert!(decode_data_url("no comma here").is_err());
    }

    #[test]
    fn a_file_that_is_not_an_image_is_refused() {
        let payload = encode(b"#!/bin/sh\nrm -rf /");
        let err = save(&format!("data:image/jpeg;base64,{payload}"));
        assert!(err.is_err());
    }
}
