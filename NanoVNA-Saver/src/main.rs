use std::thread;
use clap::Parser;
use tokio_serial::SerialPortType;
mod sweep;

#[derive(Parser, Debug)]
#[command(name = "nanovna-saver")]
#[command(about = "Performs NanoVNA sweeps with configurable parameters")]
struct Args {

    #[arg(long, default_value_t = 1)]
    num_sweeps: usize,

    #[arg(long, default_value_t = 1)]
    vna_number: usize,

    #[arg(long, default_value_t = 50_000)]
    start_freq: u64,

    #[arg(long, default_value_t = 900_000_000)]
    end_freq: u64,

    #[arg(long, default_value_t = 101)]
    num_points: usize,

    #[arg(long, default_value_t = 2)]
    num_ports: usize,

    #[arg(long)]
    if_bandwidth: Option<u32>,
}

fn main() {

    let args = Args::parse();

    let Args {
    num_sweeps,
    vna_number,
    start_freq,
    end_freq,
    mut num_points,
    num_ports,
    if_bandwidth,
    } = args;

    // Limit num_points to 101 if more are typed
    if num_points > 101 {
        println!("num_points limited to 101 (was {})", num_points);
        num_points = 101;
    }

    let ports = tokio_serial::available_ports()
        .expect("Failed to enumerate serial ports");
    
    let filtered_ports: Vec<_> = ports
        .into_iter()
        .filter(|p| {
        if let SerialPortType::UsbPort(info) = &p.port_type {
            info.vid == 0x0483 && info.pid == 0x5740
        } else {
            false
        }
    })
    .collect();

    if filtered_ports.is_empty() {
        eprintln!("No NanoVNA devices detected");
    return;
    }


    // Checks if the serial port is connected
    let vnas_to_use = filtered_ports
        .into_iter()
        .take(vna_number);
    // Print line for table header
    println!("| ID | Label | VNA NUMBER | TIME COMMAND SENT | TIME READING RECEIVED | Frequency | SParameter | Real | Imaginary |");

    let mut handles = Vec::new();

    for (idx, port) in vnas_to_use.enumerate() {
        let port_name = port.port_name.clone();
        let vna_number = idx + 1; 

        let params = sweep::SweepParams {
            port_name,
            num_sweeps,
            vna_number,
            start_freq,
            end_freq,
            num_points,
            num_ports,
            if_bandwidth,
        };
        let handle = thread::spawn(move || {
            sweep::run_on_port(params);
        });

        handles.push(handle);
    }

    for h in handles {
        h.join().unwrap();
    }
}

