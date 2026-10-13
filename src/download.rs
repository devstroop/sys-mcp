//! Atomic, verified, retrying downloads for runtime-fetched assets.
//!
//! Both model fetchers (OCR weights and YOLOv8n) previously wrote straight to
//! the final path with a single attempt and no verification. That had a
//! nastier consequence than a missing retry: an interrupted download left a
//! truncated file at the destination, and the `path.exists()` check then
//! treated it as a valid model **forever** — a permanently corrupt install
//! that no amount of retrying would repair.
//!
//! This module fixes that by downloading to a sibling temp file and renaming
//! into place only after the byte count matches the server's `Content-Length`.
//! `rename` within one directory is atomic on both Windows and Unix, so the
//! destination either does not exist or is complete.

use std::path::Path;
use std::time::Duration;

/// Download attempts before giving up (first try + retries).
const ATTEMPTS: u32 = 3;
/// Base backoff; attempt N waits `BASE * N` (2s, then 4s).
const BASE_BACKOFF: Duration = Duration::from_secs(2);

/// Download `url` to `dest`, atomically and with verification.
///
/// On failure the destination is left exactly as it was (absent, or whatever
/// was there before) and any partial temp file is removed.
pub fn download_verified(url: &str, dest: &Path, label: &str) -> Result<(), String> {
    let mut last_err = String::new();

    for attempt in 1..=ATTEMPTS {
        match attempt_once(url, dest, label) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_err = e;
                // Never leave a partial file behind between attempts.
                let _ = std::fs::remove_file(temp_path(dest));
                if attempt < ATTEMPTS {
                    let backoff = BASE_BACKOFF * attempt;
                    log::warn!(
                        "{label}: download attempt {attempt}/{ATTEMPTS} failed ({last_err}); \
                         retrying in {}s",
                        backoff.as_secs()
                    );
                    std::thread::sleep(backoff);
                }
            }
        }
    }

    Err(format!(
        "{label}: download failed after {ATTEMPTS} attempts: {last_err}"
    ))
}

/// Sibling temp file for the download. Same directory as the destination so
/// the final `rename` stays within one filesystem and is therefore atomic.
///
/// Public so tests can assert the sibling invariant directly; not intended as
/// general API.
pub fn temp_path(dest: &Path) -> std::path::PathBuf {
    let mut name = dest
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "download".to_string());
    // pid-scoped so two processes downloading the same asset cannot collide.
    name.push_str(&format!(".{}.part", std::process::id()));
    dest.with_file_name(name)
}

fn attempt_once(url: &str, dest: &Path, label: &str) -> Result<(), String> {
    let response = ureq::get(url)
        .call()
        .map_err(|e| format!("request failed: {e}"))?;

    // Content-Length is optional; when present it is the only cheap way to
    // detect a truncated body, which is exactly the failure mode that used to
    // poison the model cache.
    let content_length: Option<u64> = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok());

    // Bind the body before taking a reader: `into_body()` returns a temporary
    // that must outlive the borrow.
    let mut body = response.into_body();
    let mut reader = body.as_reader();
    let tmp = temp_path(dest);

    let written = {
        let mut file = std::fs::File::create(&tmp)
            .map_err(|e| format!("failed to create {}: {e}", tmp.display()))?;
        let copied = std::io::copy(&mut reader, &mut file)
            .map_err(|e| format!("failed to write {}: {e}", tmp.display()))?;
        // Flush to disk before the rename so a crash cannot publish a
        // zero-filled file under the final name.
        file.sync_all()
            .map_err(|e| format!("failed to flush {}: {e}", tmp.display()))?;
        copied
    };

    if let Some(expected) = content_length {
        if written != expected {
            return Err(format!(
                "truncated body: got {written} bytes, server advertised {expected}"
            ));
        }
    }
    if written == 0 {
        return Err("server returned an empty body".to_string());
    }

    std::fs::rename(&tmp, dest)
        .map_err(|e| format!("failed to move {} into place: {e}", tmp.display()))?;

    log::info!("{label}: downloaded {written} bytes to {}", dest.display());
    Ok(())
}
