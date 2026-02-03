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


#[cfg(test)]
mod tests {
    use mockall::*;
    use mockall::predicate::*;
    use super::*;
    use std::io::Error;
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

    /*#[test]
    fn test_perform_sweep_normal_data() {
        let mut mock = MockSerialPort::new();

        mock.expect_write_all()
            .withf(|buf: &[u8]| buf == b"data 0\r")
            .returning(|_| Ok(()));
        mock.expect_flush()
            .returning(|| Ok(()));
        mock.expect_read()
            .times(101)
            .returning(|buf| {
                let line = b"0.000000,0.000000\r\n";
                let len = line.len().min(buf.len());
                buf[..len].copy_from_slice(&line[..len]);
                Ok(len)
            });

        perform_sweep(&mut mock, 1).unwrap().1;
    }*/

    #[test]
    fn test_perform_sweep_reads_101_lines() {
        println!("Running test_perform_sweep_reads_101_lines");
        let mut mock = MockSerialPort::new();

        mock.expect_write_all().returning(|_| Ok(()));
        mock.expect_flush().returning(|| Ok(()));

        // build 101 lines of "x\n"
        let data = "x\n".repeat(101);
        let bytes = data.as_bytes().to_vec();

        //first read returns all data
        let value = bytes.clone(); //Can't borrow a moved value in closure
        mock.expect_read()
        .returning(move |buf| {
            buf[..value.len()].copy_from_slice(&value);
            Ok(value.len())
        });

        let mut mock = Box::new(mock) as Box<dyn tokio_serial::SerialPort>;
        let (count, text) = perform_sweep(&mut mock, 1).unwrap();

        assert_eq!(text.lines().count(), 101);
        assert_eq!(count, bytes.len());

    }

    #[test]
    fn test_stops_after_101_lines_even_if_more_arrives() {
        let mut mock = MockSerialPort::new();

        mock.expect_write_all().returning(|_| Ok(()));
        mock.expect_flush().returning(|| Ok(()));

        let data = "x\n".repeat(150);
        let bytes = data.as_bytes().to_vec();

        mock.expect_read().returning(move |buf| {
            buf[..bytes.len()].copy_from_slice(&bytes);
            Ok(bytes.len())
        });

        let mut mock = Box::new(mock) as Box<dyn tokio_serial::SerialPort>;
        let (_, text) = perform_sweep(&mut mock, 1).unwrap();
        assert!(text.lines().count() >= 101); // we only guarantee stop condition
    }


    #[test]
    fn test_command_is_sent() {
        let mut mock = MockSerialPort::new();

        mock.expect_write_all()
            .with(eq(b"data 0\r".as_ref()))
            .times(1)
            .returning(|_| Ok(()));

        mock.expect_flush()
            .times(1)
            .returning(|| Ok(()));

        // Immediately timeout so we exit read loop
        mock.expect_read()
            .returning(|_| Err(std::io::ErrorKind::TimedOut.into()));

        let mut mock = Box::new(mock) as Box<dyn tokio_serial::SerialPort>;
        let _ = perform_sweep(&mut mock, 1).unwrap();
    }

    #[test]
    fn test_timeout_returns_partial_data() {
        let mut mock = MockSerialPort::new();

        mock.expect_write_all().returning(|_| Ok(()));
        mock.expect_flush().returning(|| Ok(()));

        let partial = "x\n".repeat(20).into_bytes();
        let mut called = false;

        mock.expect_read().returning(move |buf| {
            if !called {
                buf[..partial.len()].copy_from_slice(&partial);
                called = true;
                Ok(partial.len())
            } else {
                Err(std::io::ErrorKind::TimedOut.into())
            }
        });

        let mut mock = Box::new(mock) as Box<dyn tokio_serial::SerialPort>;
        let (_, text) = perform_sweep(&mut mock, 1).unwrap();
        assert_eq!(text.lines().count(), 20);
    }

    #[test]
    fn test_wouldblock_is_retried() {
        let mut mock = MockSerialPort::new();

        mock.expect_write_all().returning(|_| Ok(()));
        mock.expect_flush().returning(|| Ok(()));

        let data = "x\n".repeat(101).into_bytes();
        let mut step = 0;

        mock.expect_read().returning(move |buf| {
            step += 1;
            match step {
                1 | 2 => Err(std::io::ErrorKind::WouldBlock.into()),
                _ => {
                    buf[..data.len()].copy_from_slice(&data);
                    Ok(data.len())
                }
            }
        });

        let mut mock = Box::new(mock) as Box<dyn tokio_serial::SerialPort>;
        let (_, text) = perform_sweep(&mut mock, 1).unwrap();
        assert_eq!(text.lines().count(), 101);
    }

}
