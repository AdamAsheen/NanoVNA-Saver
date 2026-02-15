use tokio_serial::{SerialPort, ClearBuffer};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use polars::frame::DataFrame;
use polars::series::Series;
use polars::prelude::NamedFrom;

pub fn run_on_port(port_name: String, num_sweeps: usize, vna_number: usize, num_ports: usize) {
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
    let sweep_params = args.get(4).cloned().unwrap_or_else(|| "50_000 900_000_000 101".to_string());

    let parts: Vec<&str> = sweep_params.split_whitespace().collect();
    let start_freq: u64 = parts.first().unwrap_or(&"50000").replace('_', "").parse().unwrap();
    let end_freq: u64 = parts.get(1).unwrap_or(&"900000000").replace('_', "").parse().unwrap();
    let num_points: usize = parts.get(2).unwrap_or(&"101").parse().unwrap();
    let step_freq: f64 = (end_freq - start_freq) as f64 / (num_points - 1) as f64;

    let start_time = Instant::now();
    let mut total_bytes = 0usize;
    let mut sweep_ids = Vec::new();
    let mut labels = Vec::new();
    let mut vna_numbers = Vec::new();
    let mut time_cmd_sent_vec = Vec::new();
    let mut time_received_vec = Vec::new();
    let mut frequencies = Vec::new();
    let mut channels = Vec::new();
    let mut real_parts = Vec::new();
    let mut imag_parts = Vec::new();


    for sweep_idx in 0..num_sweeps {
        let sweep_id = Uuid::new_v4();

       // for S11 port (data 0) 
        let time_cmd_sent_s11 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();

        match perform_sweep(&mut *port, 0, num_points) {
            Ok((bytes_read, sweep_data)) => {
                total_bytes += bytes_read;

                let mut point_index = 0usize;

                for line in sweep_data.lines() {
                    let line = line.trim();
                    if line.is_empty() { continue; }
                    if line.starts_with("NanoVNA") { continue; }
                    if line.starts_with("ch>") { continue; }
                    if line.starts_with("data") { continue; }

                    let mut it = line.split_whitespace();
                    let (Some(real_s), Some(imag_s)) = (it.next(), it.next()) else { continue; };
                    let (Ok(real), Ok(imag)) = (real_s.parse::<f64>(), imag_s.parse::<f64>()) else { continue; };

                    let freq = start_freq as f64 + point_index as f64 * step_freq;
                    let time_received = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64();

                    sweep_ids.push(sweep_id.to_string());
                    labels.push(label.clone());
                    vna_numbers.push(vna_number as i32);
                    time_cmd_sent_vec.push(time_cmd_sent_s11);
                    time_received_vec.push(time_received);
                    frequencies.push(freq);
                    channels.push("S11".to_string());
                    real_parts.push(real);
                    imag_parts.push(imag);

                    point_index += 1;
                    if point_index >= num_points { break; }
                }
            }
            Err(e) => {
                eprintln!("[{}] Sweep {} S11 failed: {}", port_name, sweep_idx + 1, e);
                break;
            }
        }

        // for S21 port (data 1) 
        if num_ports == 2 {
            let time_cmd_sent_s21 = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64();

            match perform_sweep(&mut *port, 1, num_points) {
                Ok((bytes_read, s21_data)) => {
                    total_bytes += bytes_read;

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
                        let time_received = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs_f64();

                        sweep_ids.push(sweep_id.to_string());
                        labels.push(label.clone());
                        vna_numbers.push(vna_number as i32);
                        time_cmd_sent_vec.push(time_cmd_sent_s21);
                        time_received_vec.push(time_received);
                        frequencies.push(freq);
                        channels.push("S21".to_string());
                        real_parts.push(real);
                        imag_parts.push(imag);

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
    }

    let elapsed = start_time.elapsed().as_secs_f64();

    let df = DataFrame::new(vec![
        Series::new("sweep_id", sweep_ids),
        Series::new("label", labels),
        Series::new("vna_number", vna_numbers),
        Series::new("time_cmd_sent", time_cmd_sent_vec),
        Series::new("time_received", time_received_vec),
        Series::new("frequency_hz", frequencies),
        Series::new("channel", channels),
        Series::new("real", real_parts),
        Series::new("imag", imag_parts),
    ]).expect("Failed to create DataFrame");

    println!("{}", df);

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
    port: &mut dyn SerialPort,
    data_idx: u8,
    num_points: usize,
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

                if total_read + 1024 > buf.len() {
                    buf.resize(buf.len() + 4096, 0);
                }

                let newline_count = buf[..total_read].iter().filter(|&&b| b == b'\n').count();
                if newline_count >= num_points { break; }

                if buf[..total_read].windows(3).any(|w| w == b"ch>") { break; }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(1));
                continue;
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => break,
            Err(e) => return Err(e),
        }
    }

    let sweep_ascii = String::from_utf8_lossy(&buf[..total_read]).to_string();
    Ok((total_read, sweep_ascii))
}
