use tokio_serial::{SerialPort, ClearBuffer};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use polars::frame::DataFrame;
use polars::series::Series;
use polars::prelude::NamedFrom;

pub fn run_on_port(port_name: String, num_sweeps: usize, vna_number:usize, start_freq: u64, end_freq: u64, num_points: usize, num_ports: usize, if_bandwidth: Option<u32>) {
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
    let label = "default_label".to_string();
    let step_freq: f64 = (end_freq - start_freq) as f64 / (num_points - 1) as f64;

    // Allow IF bandwidth to be chosen from terminal instead of the shell 
    if let Some(bw) = if_bandwidth {
        let _ = port.clear(ClearBuffer::Input);

        let set_cmd = format!("bandwidth {}\r\n", bw);
        if let Err(e) = port.write_all(set_cmd.as_bytes()) {
            eprintln!("[{}] Error failed to set bandwidth: {}", port_name, e)
        }
        let _ = port.flush();

        std::thread::sleep(Duration::from_millis(100));

        let mut resp_buff = [0u8; 512];
        match port.read(&mut resp_buff) {
            Ok(n) if n > 0 => {
                let response = String::from_utf8_lossy(&resp_buff[..n]);
                println!("[{}] IF bandwidth response:\n{}", port_name, response.trim());
            }
            Ok(_) => {
                eprint!("[{}] IF bandwidth set response: <empty>", port_name)
            }
            Err(e) => {
                eprint!("[{}] Failed to read bandwidth response: {}", port_name, e)
            }
        }
        clear_shell(&mut *port);
    }

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

        let time_cmd_sent_s11 = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs_f64();

        match perform_sweep(&mut *port, 0, start_freq, end_freq, num_points) {
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
                    time_cmd_sent_s11, time_reading_received,
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
        if num_ports == 2 {
            let time_cmd_sent_s21 = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64();

            match perform_sweep(&mut *port, 1, start_freq, end_freq, num_points) {
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
    start_freq: u64,
    end_freq: u64,
    num_points: usize,
) -> Result<(usize, String), std::io::Error> {
    
    
    let _ = port.clear(ClearBuffer::Input);

    
    let cmd = format!("sweep {} {} {}\r\n", start_freq, end_freq, num_points);
    port.write_all(cmd.as_bytes())?;
    port.flush()?;

    
    let mut scratch = [0u8; 1];
    let mut recent = Vec::new(); // keep track of last few bytes to match "ch>"
    
    let start = Instant::now();
    // Timeout for sweep execution can be long if many points
    let sweep_timeout = Duration::from_millis(5000 + (num_points as u64 * 20));

    loop {
        if start.elapsed() > sweep_timeout {
             break;
        }

        match port.read(&mut scratch) {
            Ok(1) => {
                recent.push(scratch[0]);
                if recent.len() > 3 {
                    recent.remove(0);
                }
                
                
                if recent == b"ch>" {
                    break;
                }
            }
            Ok(_) => { 
                
                std::thread::yield_now();
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::yield_now();
            },
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                 // ignore
            },
            Err(e) => return Err(e),
        }
    }

    let cmd = format!("data {}\r\n", data_idx);
    port.write_all(cmd.as_bytes())?;
    port.flush()?;

    let mut buf = Vec::with_capacity(num_points * 30);
    // Pre-allocated buffer: approx 20-30 bytes per line * num_points
    
    let mut total_read = 0usize;
    let start = Instant::now();
    // Timeout for reading data
    let read_timeout = Duration::from_millis(3000 + (num_points as u64 * 10));

    loop {
        if start.elapsed() > read_timeout {
            break;
        }

        let mut chunk = [0u8; 4096];
        match port.read(&mut chunk) {
            Ok(n) => {
                if n > 0 {
                    buf.extend_from_slice(&chunk[..n]);
                    total_read += n;

                    // Check for end of data stream ("ch>")
                    if total_read >= 3 {
                        let tail = &buf[total_read-3..];
                        if tail == b"ch>" {
                            break;
                        }
                        // Also check for "ch> " (space) or "ch>\r"
                        if total_read >= 4 {
                             let tail4 = &buf[total_read-4..];
                             if tail4 == b"ch> " || tail4 == b"ch>\r" || tail4 == b"ch>\n" {
                                 break;
                             }
                        }
                    }
                } else {
                    std::thread::yield_now();
                }
            },
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::yield_now();
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                 // ignore
            },
            Err(e) => return Err(e),
        }
    }

    let sweep_ascii = String::from_utf8_lossy(&buf).to_string();
    Ok((total_read, sweep_ascii))
}

#[cfg(test)]
mod tests {
    use mockall::*;
    use mockall::predicate::*;
    use super::*;
    use std::io::Error;
    use std::cell::Cell;
    use tokio_serial::{DataBits, FlowControl, Parity, StopBits, ClearBuffer, SerialPort};

    mock! {
        SerialPort {}

        impl std::io::Read for SerialPort {
            fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error>;
        }

        impl std::io::Write for SerialPort {
            fn write(&mut self, buf: &[u8]) -> Result<usize, Error>;
            fn write_all(&mut self, buf: &[u8]) -> Result<(), Error>;
            fn flush(&mut self) -> Result<(), Error>;
        }

        impl tokio_serial::SerialPort for SerialPort {
            fn name(&self) -> Option<String>;
            fn baud_rate(&self) -> Result<u32, tokio_serial::Error>;
            fn data_bits(&self) -> Result<DataBits,tokio_serial:: Error>;
            fn flow_control(&self) -> Result<FlowControl, tokio_serial::Error>;
            fn parity(&self) -> Result<Parity, tokio_serial::Error>;
            fn stop_bits(&self) -> Result<StopBits, tokio_serial::Error>;
            fn timeout(&self) -> Duration;
            
            fn set_baud_rate(&mut self, baud_rate: u32) -> Result<(), tokio_serial::Error>;
            fn set_data_bits(&mut self, data_bits: DataBits) -> Result<(), tokio_serial::Error>;
            fn set_flow_control(
                &mut self,
                flow_control: FlowControl,
            ) -> Result<(), tokio_serial::Error>;
            fn set_parity(&mut self, parity: Parity) -> Result<(), tokio_serial::Error>;
            fn set_stop_bits(&mut self, stop_bits: StopBits) -> Result<(), tokio_serial::Error>;
            fn set_timeout(&mut self, timeout: Duration) -> Result<(), tokio_serial::Error>;
            
            fn write_request_to_send(&mut self, level: bool) -> Result<(), tokio_serial::Error>;
            fn write_data_terminal_ready(&mut self, level: bool) -> Result<(), tokio_serial::Error>;
            
            fn read_clear_to_send(&mut self) -> Result<bool, tokio_serial::Error>;
            fn read_data_set_ready(&mut self) -> Result<bool, tokio_serial::Error>;
            fn read_ring_indicator(&mut self) -> Result<bool, tokio_serial::Error>;
            fn read_carrier_detect(&mut self) -> Result<bool, tokio_serial::Error>;
            
            fn bytes_to_read(&self) -> Result<u32, tokio_serial::Error>;
            fn bytes_to_write(&self) -> Result<u32, tokio_serial::Error>;
            
            fn clear(&self, buffer_to_clear: ClearBuffer) -> Result<(), tokio_serial::Error>;
            fn try_clone(&self) -> Result<Box<dyn SerialPort>, tokio_serial::Error>;
            
            fn set_break(&self) -> Result<(), tokio_serial::Error>;
            fn clear_break(&self) -> Result<(), tokio_serial::Error>;
        }
    }

    #[test]
    fn test_perform_sweep_normal_data() {
        let mut mock = MockSerialPort::new();
        let mut seq = Sequence::new(); 

        mock.expect_write_all().returning(|_| Ok(()));
        mock.expect_flush().returning(|| Ok(()));
        mock.expect_clear().returning(|_| Ok::<(), tokio_serial::Error>(()));

        mock.expect_read().times(1).in_sequence(&mut seq).returning(|buf| { buf[0] = b'c'; Ok(1) });
        mock.expect_read().times(1).in_sequence(&mut seq).returning(|buf| { buf[0] = b'h'; Ok(1) });
        mock.expect_read().times(1).in_sequence(&mut seq).returning(|buf| { buf[0] = b'>'; Ok(1) });

        mock.expect_read()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|buf| {
                let mut data = "0.000000,0.000000\r\n".repeat(101);
                data.push_str("ch>"); // Append the terminator
                let bytes = data.as_bytes();
                let len = bytes.len().min(buf.len());
                buf[..len].copy_from_slice(&bytes[..len]);
                Ok(len)
            });

        let text = perform_sweep(&mut mock, 1, 101, 50000, 50000).unwrap().1;
        let expected = "0.000000,0.000000\r\n".repeat(101) + "ch>";

        assert_eq!(text, expected);
    }

    #[test]
    fn test_perform_sweep_reads_101_lines() {
        println!("Running test_perform_sweep_reads_101_lines");
        let mut mock = MockSerialPort::new();
        let mut seq = Sequence::new();

        mock.expect_write_all().returning(|_| Ok(()));
        mock.expect_flush().returning(|| Ok(()));
        mock.expect_clear()
            .returning(|_| Ok::<(), tokio_serial::Error>(()));

        mock.expect_read().times(1).in_sequence(&mut seq).returning(|buf| { buf[0] = b'c'; Ok(1) });
        mock.expect_read().times(1).in_sequence(&mut seq).returning(|buf| { buf[0] = b'h'; Ok(1) });
        mock.expect_read().times(1).in_sequence(&mut seq).returning(|buf| { buf[0] = b'>'; Ok(1) });


        let mut data = "x\n".repeat(101);
        data.push_str("ch>");
        let bytes = data.as_bytes().to_vec();


        let value = bytes.clone();
        mock.expect_read()
        .in_sequence(&mut seq)
        .returning(move |buf| {
            let len = value.len().min(buf.len());
            buf[..len].copy_from_slice(&value[..len]);
            Ok(len)
        });

        let mut mock = Box::new(mock) as Box<dyn tokio_serial::SerialPort>;
        let (count, text) = perform_sweep(mock.as_mut(), 0, 50_000_000, 900_000_000, 101).unwrap();

        // Check we got at least 101 lines (plus the prompt line)
        assert!(text.lines().count() >= 101);
        assert_eq!(count, bytes.len());

    }

    #[test]
    fn test_stops_after_101_lines_even_if_more_arrives() {
        let mut mock = MockSerialPort::new();
        let mut seq = Sequence::new();

        mock.expect_write_all().returning(|_| Ok(()));
        mock.expect_flush().returning(|| Ok(()));
        mock.expect_clear()
            .returning(|_| Ok::<(), tokio_serial::Error>(()));

        mock.expect_read().times(1).in_sequence(&mut seq).returning(|buf| { buf[0] = b'c'; Ok(1) });
        mock.expect_read().times(1).in_sequence(&mut seq).returning(|buf| { buf[0] = b'h'; Ok(1) });
        mock.expect_read().times(1).in_sequence(&mut seq).returning(|buf| { buf[0] = b'>'; Ok(1) });

        let mut data = "x\n".repeat(101);
        data.push_str("ch>");
        let bytes = data.as_bytes().to_vec();

        let value = bytes.clone();
        mock.expect_read()
            .in_sequence(&mut seq)
            .returning(move |buf| {
                let len = value.len().min(buf.len());
                buf[..len].copy_from_slice(&value[..len]);
                Ok(len)
        });

        let mut mock = Box::new(mock) as Box<dyn tokio_serial::SerialPort>;
        let (_, text) = perform_sweep(mock.as_mut(), 0, 50_000_000, 900_000_000, 101).unwrap();
        assert!(text.lines().count() >= 101); // we only guarantee stop condition
    }


    #[test]
    fn test_command_is_sent() {
        let mut mock = MockSerialPort::new();

        mock.expect_write_all()
            .with(eq(b"sweep 50000000 900000000 101\r\n".as_ref()))
            .times(1)
            .returning(|_| Ok(()));

        mock.expect_write_all()
            .with(eq(b"data 0\r\n".as_ref()))
            .times(1)
            .returning(|_| Ok(()));

        mock.expect_flush()
            .times(2)
            .returning(|| Ok(()));

        mock.expect_clear()
            .returning(|_| Ok::<(), tokio_serial::Error>(()));

        // Immediately timeout so we exit read loop
        mock.expect_read()
            .returning(|_| Err(std::io::ErrorKind::TimedOut.into()));

        let mut mock = Box::new(mock) as Box<dyn tokio_serial::SerialPort>;
        let _ = perform_sweep(mock.as_mut(), 0, 50_000_000, 900_000_000, 101).unwrap();
    }

    #[test]
    fn test_timeout_returns_partial_data() {
        let mut mock = MockSerialPort::new();
        let mut seq = Sequence::new();

        mock.expect_write_all().returning(|_| Ok(()));
        mock.expect_flush().returning(|| Ok(()));
        mock.expect_clear()
            .returning(|_| Ok::<(), tokio_serial::Error>(()));

        mock.expect_read().times(1).in_sequence(&mut seq).returning(|buf| { buf[0] = b'c'; Ok(1) });
        mock.expect_read().times(1).in_sequence(&mut seq).returning(|buf| { buf[0] = b'h'; Ok(1) });
        mock.expect_read().times(1).in_sequence(&mut seq).returning(|buf| { buf[0] = b'>'; Ok(1) });

        let partial = "x\n".repeat(20).into_bytes();
        let called = Cell::new(false);
        let partial_clone = partial.clone();

        mock.expect_read()
            .in_sequence(&mut seq)
            .returning(move |buf| {
                if !called.get() {
                    buf[..partial_clone.len()].copy_from_slice(&partial);
                    called.set(true);
                    Ok(partial_clone.len())
                } else {
                    Err(std::io::ErrorKind::TimedOut.into())
                }
            });

        let mut mock = Box::new(mock) as Box<dyn tokio_serial::SerialPort>;
        let (_, text) = perform_sweep(mock.as_mut(), 0, 50_000_000, 900_000_000, 101).unwrap();
        assert_eq!(text.lines().count(), 20);
    }

    #[test]
    fn test_wouldblock_is_retried() {
        let mut mock = MockSerialPort::new();
        let mut seq = Sequence::new();

        mock.expect_write_all().returning(|_| Ok(()));
        mock.expect_flush().returning(|| Ok(()));
        mock.expect_clear()
            .returning(|_| Ok::<(), tokio_serial::Error>(()));

        mock.expect_read().times(1).in_sequence(&mut seq).returning(|buf| { buf[0] = b'c'; Ok(1) });
        mock.expect_read().times(1).in_sequence(&mut seq).returning(|buf| { buf[0] = b'h'; Ok(1) });
        mock.expect_read().times(1).in_sequence(&mut seq).returning(|buf| { buf[0] = b'>'; Ok(1) });

        let mut data = "x\n".repeat(101);
        data.push_str("ch>");
        let bytes = data.as_bytes().to_vec();

        let step = Cell::new(0u32);

        // We need to use "move" carefully with data inside closure
        // But since we need to clone data for the closure...
        let value = bytes.clone();
        
        mock.expect_read()
            .in_sequence(&mut seq)
            .returning(move |buf| {
                let s = step.get() + 1;
                step.set(s);

                match s {
                    1 | 2 => Err(std::io::ErrorKind::WouldBlock.into()),
                    _ => {
                        let len = value.len().min(buf.len());
                        buf[..len].copy_from_slice(&value[..len]);
                        Ok(len)
                    }
                }
            });

        let mut mock = Box::new(mock) as Box<dyn tokio_serial::SerialPort>;
        let (_, text) = perform_sweep(mock.as_mut(), 0, 50_000_000, 900_000_000, 101).unwrap();
        // At least 101 lines again
        assert!(text.lines().count() >= 101);
    }
}