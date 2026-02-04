use tokio_serial::{SerialPort, SerialPortBuilderExt, ClearBuffer};
use std::io::{Read, Write};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub fn run_on_port(port_name: String, num_sweeps: usize, vna_number:usize) {
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

    // Seperation of titles and headers 
    let args: Vec<String> = std::env::args().collect();
    let label = "default_label".to_string();
    let sweep_params = args.get(3).cloned().unwrap_or_else(|| "50_000 900_000_000 101".to_string());

    let parts: Vec<&str> = sweep_params.split_whitespace().collect();
    let start_freq: u64 = parts.get(0).unwrap_or(&"50000").replace('_', "").parse().unwrap();
    let end_freq: u64 = parts.get(1).unwrap_or(&"900000000").replace('_', "").parse().unwrap();
    let num_points: usize = parts.get(2).unwrap_or(&"101").parse().unwrap();
    let step_freq: f64 = (end_freq - start_freq) as f64 / (num_points - 1) as f64;

    let start_time = Instant::now();
    let mut total_bytes = 0usize;

    for sweep_idx in 0..num_sweeps {
        let sweep_id = Uuid::new_v4();
            let time_cmd_sent = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64();
        
        // for S11 port (data 0)
        match perform_sweep(&mut *port, 0 , num_points) {
            Ok((bytes_read, sweep_data)) => {
                total_bytes += bytes_read;

                println!(
                    "[{}] Sweep {} complete ({} bytes)",
                    port_name,
                    sweep_idx + 1,
                    bytes_read
                );

            let mut point_index =0usize;

            for line in sweep_data.lines() {
                let line = line.trim();
                if line.is_empty() { continue; }
                if line.starts_with("NanoVNA") { continue; }
                if line.starts_with("ch>") { continue; }
                if line.starts_with("data") { continue; }

                let mut it = line.split_whitespace();
                let (Some(real_s), Some(imag_s)) = (it.next(), it.next()) else { continue; };

                let (Ok(real), Ok(imag)) = 
                    (real_s.parse::<f64>(), imag_s.parse::<f64>()) else { continue; }; 

                let freq = start_freq as f64 + point_index as f64 * step_freq;
                let time_reading_received = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs_f64();

                println!("| {} | {} | {} | {:.6} | {:.6} | {:.0} | S11 | {} | {} |",
                    sweep_id, label, vna_number,
                    time_cmd_sent, time_reading_received,
                    freq, real, imag
                );

                point_index += 1;
                if point_index >= num_points {
                    break;
                }
            }

            }
            Err(e) => {
                eprintln!(
                    "[{}] Sweep {} S11 failed: {}",
                    port_name,
                    sweep_idx + 1,
                    e
                );
                break;
            }
        }
        // for S21 port (data 1)
        match perform_sweep(&mut *port, 1, num_points) {
            Ok((bytes_read, s21_data)) => {
                total_bytes += bytes_read;
                println!(
                    "[{}] Sweep {} S21 complete ({} bytes)",
                    port_name,
                    sweep_idx + 1,
                    bytes_read
                );
                let mut point_index = 0usize;
                for line in s21_data.lines() {
                    let line = line.trim();
                    if line.is_empty() { continue; }
                    if line.starts_with("NanoVNA") { continue; }
                    if line.starts_with("ch>") { break; }
                    if line.starts_with("data") { continue; }

                    let mut it = line.split_whitespace();
                    let (Some(real_s), Some(imag_s)) = (it.next(), it.next()) else { continue; };
                    let (Ok(real), Ok(imag)) = (real_s.parse::<f64>(), imag_s.parse::<f64>()) else { continue; };

                    let freq = start_freq as f64 + point_index as f64 * step_freq;
                    let time_reading_received = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64();

                    println!(
                        "| {} | {} | {} | {:.6} | {:.6} | {:.0} | S21 | {} | {} |",
                        sweep_id, label, vna_number,
                        time_cmd_sent, time_reading_received,
                        freq, real, imag
                    );

                    point_index += 1;
                    if point_index >= num_points { break; }
                }

            }
            Err(e) => {
                eprintln!("[{}] Sweep {} S21 failed: {}", port_name, sweep_idx + 1, e);
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
    let _ = port.clear(ClearBuffer::Input);
}

fn perform_sweep(
    port: &mut dyn SerialPort, data_idx: u8, num_points: usize, 
) -> Result<(usize, String), std::io::Error> {
    
    let _ = port.clear(ClearBuffer::Input); 
    let cmd = format!("data {}\r\n", data_idx);
    port.write_all(cmd.as_bytes())?;
    port.flush()?;

    let mut buf = vec![0u8; 4096];
    let mut total_read = 0usize;

    let start = Instant::now();
    let max_duration = Duration::from_millis(500);
    let max_bytes = 32 * 1024;

    while start.elapsed() < max_duration && total_read < max_bytes {
        match port.read(&mut buf[total_read..]) {
            Ok(0) => break,
            Ok(n) => {
                total_read += n;

                if total_read + 1024 > buf.len(){
                    buf.resize(buf.len() + 4096, 0);
                }

                let newline_count = buf[..total_read].iter().filter(|&&b| b == b'\n').count();
                if newline_count >= num_points {
                    break;
                }

                if buf[..total_read].windows(3).any(|w| w == b"ch>") {
                    break;
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

