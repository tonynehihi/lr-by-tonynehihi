//! HEIC/HEIF decoding for the macOS build of this fork ("Lightroom by Tony").
//!
//! There is no pure-Rust, permissively licensed HEVC decoder, so on macOS the
//! operating system's own decoder does the work: the file is staged in a private
//! temp folder, converted to TIFF by `sips` (part of every macOS install), and
//! the TIFF is decoded by the normal TIFF path. Nothing is linked against the
//! system codec and no data leaves the machine. Other platforms keep returning
//! `Error::Unsupported` for HEIF.
//!
//! Note: the conversion yields the system's SDR rendering of the photo; an HDR
//! gain map inside the HEIC is not carried over.

use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{DecodeOptions, Decoded, Error, Format, Result};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Decode HEIC/HEIF bytes via the macOS system converter, as described above.
pub fn decode(bytes: &[u8], opts: &DecodeOptions) -> Result<Decoded> {
    let dir = TempDir::new()?;
    let src = dir.0.join("in.heic");
    let dst = dir.0.join("out.tiff");
    std::fs::File::create(&src)
        .and_then(|mut f| f.write_all(bytes))
        .map_err(|e| malformed(format!("could not stage the HEIC file: {e}")))?;
    let out = std::process::Command::new("/usr/bin/sips")
        .arg("-s")
        .arg("format")
        .arg("tiff")
        .arg(&src)
        .arg("--out")
        .arg(&dst)
        .output()
        .map_err(|e| malformed(format!("could not run the macOS image converter: {e}")))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let tail: String = stderr.trim().chars().rev().take(300).collect::<String>().chars().rev().collect();
        return Err(malformed(format!("the macOS image converter could not read this HEIC file: {tail}")));
    }
    let tiff = std::fs::read(&dst).map_err(|e| malformed(format!("could not read the converted image: {e}")))?;
    let mut d = crate::tiff_codec::decode(&tiff, opts)?;
    d.format = Format::Heif;
    Ok(d)
}

fn malformed(msg: String) -> Error {
    Error::Malformed(Format::Heif, msg)
}

/// A private temp folder that removes itself on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Result<Self> {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("lightcraft-heic-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|e| malformed(format!("could not create a temp folder: {e}")))?;
        Ok(TempDir(dir))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn garbage_bytes_error_instead_of_panicking() {
        let opts = DecodeOptions::default();
        assert!(decode(b"this is not a heic file", &opts).is_err());
        assert!(decode(&[], &opts).is_err());
    }
}
