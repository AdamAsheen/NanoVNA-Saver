use tokio_serial::{SerialPort, SerialPortBuilderExt};
use std::io::{Read, Write};
use std::time::{Duration, Instant};

pub fn run_on_port(port_name: String, num_sweeps: usize) {
    println!("[{}] Starting VNA worker", port_name);

    let builder = tokio_serial::new(&port_name, 115200)
        .timeout(Duration::from_millis(100));

    let mut port = match builder.open() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[{}] Failed to open port: {}", port_name, e);
            return;
        }
    };

    clear_shell(&mut *port);

    let start_time = Instant::now();
    let mut total_bytes = 0usize;

    for sweep_idx in 0..num_sweeps {
        match perform_sweep(&mut *port) {
            Ok((bytes_read, sweep_data)) => {
                total_bytes += bytes_read;

                println!(
                    "[{}] Sweep {} complete ({} bytes)",
                    port_name,
                    sweep_idx + 1,
                    bytes_read
                );

                println!(
                    "[{}] Sweep {} data:\n{}",
                    port_name,
                    sweep_idx + 1,
                    sweep_data
                );
            }
            Err(e) => {
                eprintln!(
                    "[{}] Sweep {} failed: {}",
                    port_name,
                    sweep_idx + 1,
                    e
                );
                break;
            }
        }
    }

    let elapsed = start_time.elapsed().as_secs_f64();

    println!(
        "[{}] Finished: {} sweeps, {} bytes, {:.2}s",
        port_name,
        num_sweeps,
        total_bytes,
        elapsed
    );
}

fn clear_shell(port: &mut dyn SerialPort) {
    let mut buf = [0u8; 512];
    let _ = port.read(&mut buf);
}

fn perform_sweep(
    port: &mut dyn SerialPort,
) -> Result<(usize, String), std::io::Error> {
    port.write_all(b"data 0\r")?;
    port.flush()?;

    let mut buf = vec![0u8; 4096];
    let mut total_read = 0usize;
    let mut newline_count = 0usize;

    let start = Instant::now();
    let max_duration = Duration::from_millis(500);
    let max_bytes = 32 * 1024;

    while start.elapsed() < max_duration && total_read < max_bytes {
        match port.read(&mut buf[total_read..]) {
            Ok(0) => break,
            Ok(n) => {
                total_read += n;

                newline_count = buf[..total_read]
                    .iter()
                    .filter(|&&b| b == b'\n')
                    .count();

                if newline_count >= 101 {
                    break;
                }

                if total_read + 1024 > buf.len() {
                    buf.resize(buf.len() + 4096, 0);
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                break;
            }
            Err(e) => return Err(e),
        }
    }

    let sweep_ascii = String::from_utf8_lossy(&buf[..total_read]).to_string();
    Ok((total_read, sweep_ascii))
}

