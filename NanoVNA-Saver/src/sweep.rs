use tokio_serial;
use std::io::{Read, Write};
use std::time::{Duration, Instant};

fn main() {
    // Parse command line arguments for number of sweeps
    let args: Vec<String> = std::env::args().collect();
    let num_sweeps = args.get(1)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);

    if let Ok(ports) = tokio_serial::available_ports() {
        if let Some(p) = ports.first() {
            println!("Using port: {}", p.port_name);

            let builder = tokio_serial::new(&p.port_name, 115200)
                .timeout(Duration::from_secs(2));
            
            match builder.open() {
                Ok(mut port) => {
                    let mut total_bytes = 0;
                    let start_time = Instant::now();

                    // Perform continuous sweeps
                    for i in 0..num_sweeps {
                        match perform_sweep(&mut port, i + 1) {
                            Ok((bytes_read, sweep_data)) => {
                                total_bytes += bytes_read;
                                println!("Sweep {}: Read {} bytes ({} lines)", i + 1, bytes_read, sweep_data.lines().count());
                                println!("  Data (ASCII): {}", sweep_data.escape_debug());
                            }
                            Err(e) => {
                                eprintln!("Sweep {} failed: {}", i + 1, e);
                                break;
                            }
                        }
                    }

                    let elapsed = start_time.elapsed();

                    // Print summary statistics
                    println!("\n=== SWEEP SUMMARY ===");
                    println!("Sweeps completed: {}", num_sweeps);
                    println!("Total bytes read: {}", total_bytes);
                    println!("Total time: {:.6} seconds", elapsed.as_secs_f64());
                    println!("Average time per sweep: {:.6} seconds", 
                             elapsed.as_secs_f64() / num_sweeps as f64);
                    println!("Throughput: {:.2} KB/s", 
                             (total_bytes as f64 / elapsed.as_secs_f64()) / 1024.0);
                }
                Err(e) => {
                    eprintln!("Failed to open port: {}", e);
                    std::process::exit(1);
                }
            }
        } else {
            eprintln!("No serial ports found");
            std::process::exit(1);
        }
    } else {
        eprintln!("Failed to enumerate serial ports");
        std::process::exit(1);
    }
}

fn perform_sweep(port: &mut Box<dyn tokio_serial::SerialPort>, _: usize) -> Result<(usize, String), std::io::Error> {


    port.write_all(b"data 0\r")?;
    port.flush()?;

    // Read until we have complete sweep data (101 lines)
    let mut buf = vec![0u8; 2800];
    let mut total_read = 0;
    let mut line_count = 0;

    while line_count < 101 && total_read < buf.len() {
        match port.read(&mut buf[total_read..]) {
            Ok(n) if n > 0 => {
                total_read += n;
                // Count newlines in the data we just read
                line_count = buf[..total_read].iter().filter(|&&b| b == b'\n').count();
                // Expand buffer if needed
                if total_read >= buf.len() - 1024 {
                    buf.resize(buf.len() + 4096, 0);
                }
            }
            Ok(_) => break,
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => break,
            Err(e) => return Err(e),
        }
    }

    let sweep_ascii = String::from_utf8_lossy(&buf[..total_read]).to_string();
    
    Ok((total_read, sweep_ascii))
}
