//! Tests for the atomic/verified model downloader.
//!
//! The bug this guards: both model fetchers used to write straight to the
//! final path, so an interrupted download left a truncated file that
//! `path.exists()` then reported as a valid model **forever**. These use a
//! local HTTP stub so no network is involved.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Minimal HTTP/1.1 stub. Each connection pops the next scripted response.
///
/// Reads the request to its header terminator before replying: writing and
/// closing first races with the client still sending, which surfaces as
/// WSAECONNABORTED (10053) rather than as the behaviour under test.
fn spawn_server(responses: Vec<String>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let idx = Arc::new(AtomicUsize::new(0));
    let scripts = Arc::new(responses);

    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut s) = stream else { break };
            let _ = s.set_read_timeout(Some(Duration::from_secs(2)));

            // Drain the request head so we reply to a fully-received request.
            let mut head = Vec::new();
            let mut buf = [0u8; 1024];
            while !head.windows(4).any(|w| w == b"\r\n\r\n") {
                match s.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => head.extend_from_slice(&buf[..n]),
                }
            }

            let i = idx.fetch_add(1, Ordering::SeqCst);
            // Clamp to the last scripted response: a client may open an extra
            // connection, and single-response tests should still exercise the
            // behaviour under test rather than the fallback 500.
            let resp = {
                let last = scripts.len().saturating_sub(1);
                scripts.get(i.min(last)).cloned().unwrap_or_else(|| {
                    "HTTP/1.1 500 Server Error\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
                        .into()
                })
            };
            let _ = s.write_all(resp.as_bytes());
            let _ = s.flush();
        }
    });

    format!("http://{addr}/model.bin")
}

fn tmp_dest(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sys-mcp-dl-test-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("mkdir");
    dir.join("model.bin")
}

#[test]
fn successful_download_is_atomic_and_leaves_no_temp_file() {
    let body = b"MODEL-BYTES-12345";
    let url = spawn_server(vec![format!(
        "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        body.len(),
        String::from_utf8_lossy(body)
    )]);
    let dest = tmp_dest("ok");

    sys_mcp::download::download_verified(&url, &dest, "test model").expect("download");

    assert_eq!(std::fs::read(&dest).expect("read dest"), body);
    // The atomic guarantee: no partial artifact survives a success.
    let leftovers: Vec<_> = std::fs::read_dir(dest.parent().unwrap())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains(".part"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "temp files left behind: {leftovers:?}"
    );

    let _ = std::fs::remove_dir_all(dest.parent().unwrap());
}

/// The regression that motivated the rewrite: a short body must be detected
/// and must NOT be published at the destination path.
#[test]
fn truncated_body_is_rejected_and_nothing_is_published() {
    // Advertise 100 bytes, send 5. Server closes without the rest.
    let url = spawn_server(vec![format!(
        "HTTP/1.1 200 OK\r\ncontent-length: 100\r\nconnection: close\r\n\r\n{}",
        "SHORT"
    )]);
    let dest = tmp_dest("trunc");

    let err = sys_mcp::download::download_verified(&url, &dest, "test model")
        .expect_err("truncation must fail");

    assert!(
        err.contains("truncated") || err.contains("after 3 attempts"),
        "unexpected error: {err}"
    );
    assert!(
        !dest.exists(),
        "a truncated download must never be published at the destination — \
         this is the bug that permanently corrupted the model cache"
    );

    let _ = std::fs::remove_dir_all(dest.parent().unwrap());
}

#[test]
fn empty_body_is_rejected() {
    let url = spawn_server(vec![
        "HTTP/1.1 200 OK\r\ncontent-length: 0\r\nconnection: close\r\n\r\n".to_string(),
    ]);
    let dest = tmp_dest("empty");

    let err = sys_mcp::download::download_verified(&url, &dest, "test model")
        .expect_err("empty body must fail");
    assert!(err.contains("empty"), "unexpected error: {err}");
    assert!(!dest.exists());

    let _ = std::fs::remove_dir_all(dest.parent().unwrap());
}

#[test]
fn retries_a_failed_attempt_then_succeeds() {
    let body = b"RECOVERED";
    // Attempt 1: connection closed with no response at all. Attempt 2: OK.
    let url = spawn_server(vec![
        String::new(), // server writes nothing and closes -> request fails
        format!(
            "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            String::from_utf8_lossy(body)
        ),
    ]);
    let dest = tmp_dest("retry");

    sys_mcp::download::download_verified(&url, &dest, "test model").expect("retry then succeed");

    assert_eq!(std::fs::read(&dest).expect("read dest"), body);

    let _ = std::fs::remove_dir_all(dest.parent().unwrap());
}

#[test]
fn temp_path_stays_in_the_destination_directory() {
    // rename() is only atomic within one filesystem, so the temp file must be
    // a sibling of the destination, not in a system temp dir.
    let dest = PathBuf::from("/some/dir/model.bin");
    let tmp = sys_mcp::download::temp_path(&dest);
    assert_eq!(tmp.parent(), dest.parent(), "temp file must be a sibling");
    assert_ne!(tmp, dest);
    assert!(tmp.to_string_lossy().contains(".part"));
}
