//! Test input server for automated UI testing without moving the real cursor.
//!
//! Listens on a local TCP port and accepts JSON commands to inject synthetic
//! pointer events into the egui event loop.

use std::io::Read;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

/// A synthetic input event to inject into the egui loop.
pub enum TestInput {
    Click { x: f32, y: f32 },
    DoubleClick { x: f32, y: f32 },
    RightClick { x: f32, y: f32 },
}

/// Thread-safe queue of pending test inputs.
pub type TestInputQueue = Arc<Mutex<Vec<TestInput>>>;

/// Create a new empty test input queue.
pub fn create_queue() -> TestInputQueue {
    Arc::new(Mutex::new(Vec::new()))
}

/// Start a TCP server on `127.0.0.1:19400` that accepts JSON test commands.
///
/// Each connection can send one JSON object per line:
/// ```json
/// {"type":"click","x":100,"y":200}
/// {"type":"double_click","x":100,"y":200}
/// {"type":"right_click","x":100,"y":200}
/// ```
pub fn start_server(queue: TestInputQueue) {
    std::thread::spawn(move || {
        let listener = match TcpListener::bind("127.0.0.1:19400") {
            Ok(l) => l,
            Err(e) => {
                tracing::warn!("Test server failed to bind: {e}");
                return;
            }
        };
        tracing::info!("Test input server listening on 127.0.0.1:19400");

        for stream in listener.incoming() {
            let mut stream = match stream {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("Test server accept error: {e}");
                    continue;
                }
            };

            let mut buf = String::new();
            if let Err(e) = stream.read_to_string(&mut buf) {
                tracing::warn!("Test server read error: {e}");
                continue;
            }

            for line in buf.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                match parse_test_input(line) {
                    Some(input) => {
                        if let Ok(mut q) = queue.lock() {
                            q.push(input);
                        }
                    }
                    None => {
                        tracing::warn!("Test server: unrecognised input: {line}");
                    }
                }
            }
        }
    });
}

fn parse_test_input(json: &str) -> Option<TestInput> {
    let x = extract_f32(json, "x")?;
    let y = extract_f32(json, "y")?;

    if json.contains("\"double_click\"") {
        Some(TestInput::DoubleClick { x, y })
    } else if json.contains("\"right_click\"") {
        Some(TestInput::RightClick { x, y })
    } else if json.contains("\"click\"") {
        Some(TestInput::Click { x, y })
    } else {
        None
    }
}

fn extract_f32(json: &str, key: &str) -> Option<f32> {
    let pattern = format!("\"{key}\"");
    let idx = json.find(&pattern)?;
    let rest = &json[idx + pattern.len()..];
    let rest = rest.trim_start().strip_prefix(':')?;
    let rest = rest.trim_start();
    let end = rest
        .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}
