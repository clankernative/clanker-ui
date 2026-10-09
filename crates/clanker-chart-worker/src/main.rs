mod engine;
use clanker_chart_worker::{contracts, process_error, process_with, FRAME_LIMIT};
use engine::{Echarts, ENGINE_SHA256};
use sha2::{Digest, Sha256};
use std::io::{self, BufRead, Write};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["--contracts"] {
        println!("{}", contracts());
        return;
    }
    if args == ["--licenses"] {
        print!("{}", include_str!("../vendor/echarts/NOTICE"));
        println!(
            "\nECharts 6.0.0 SHA-256: {ENGINE_SHA256}\n\n{}",
            include_str!("../vendor/echarts/LICENSE")
        );
        print!("\n{}", include_str!("../vendor/DEPENDENCY-NOTICES.txt"));
        return;
    }
    if !args.is_empty() {
        eprintln!("usage: clanker-chart-worker [--contracts|--licenses]");
        std::process::exit(2);
    }
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut stdout = io::BufWriter::new(io::stdout().lock());
    let mut frame = Vec::new();
    let mut hash = Sha256::new();
    let mut oversized = false;
    loop {
        let (chunk, consumed, newline) = match input.fill_buf() {
            Ok([]) => {
                if !frame.is_empty() || oversized {
                    let id = format!("sha256:{:x}", hash.clone().finalize());
                    let out = if oversized {
                        process_error(id, "frame_limit")
                    } else {
                        process_with(&frame, &Echarts)
                    };
                    let _ = stdout.write_all(&out).and_then(|_| stdout.write_all(b"\n"));
                }
                break;
            }
            Ok(buf) => {
                let len = buf.iter().position(|b| *b == b'\n').unwrap_or(buf.len());
                (
                    buf[..len].to_vec(),
                    len + usize::from(len < buf.len()),
                    len < buf.len(),
                )
            }
            Err(_) => break,
        };
        hash.update(&chunk);
        if !oversized {
            if frame.len().saturating_add(chunk.len()) <= FRAME_LIMIT {
                frame.extend_from_slice(&chunk);
            } else {
                oversized = true;
                frame.clear();
            }
        }
        input.consume(consumed);
        if newline {
            let id = format!("sha256:{:x}", hash.clone().finalize());
            let out = if oversized {
                process_error(id, "frame_limit")
            } else {
                process_with(&frame, &Echarts)
            };
            if stdout
                .write_all(&out)
                .and_then(|_| stdout.write_all(b"\n"))
                .is_err()
            {
                break;
            }
            let _ = stdout.flush();
            frame.clear();
            hash = Sha256::new();
            oversized = false;
        }
    }
}
