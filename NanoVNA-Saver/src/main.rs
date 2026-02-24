use std::thread;
use clap::Parser;
mod sweep;

    //********************************************************************************************************
    //********************************************************************************************************
    //******************************************Command Line Format*******************************************
    // *******************************************************************************************************
    //cargo run [number_of_sweeps] [vna_number] [start_freq] [end_freq] [num_points] [num_ports] [if_bandwidth]
    //*****defaults to 1 sweep, 1 vna, start frequency 50kHz, end frequency 900MHz, number of points 101******
    //********************************************************************************************************
    //************Maximum number of points is 101, if the input number is more it will default to 101*********
    //********************************************************************************************************
    //************************************************Example*************************************************
    //**********************************cargo run 5 1 50_000 900_000_000 101**********************************
    //********************************************************************************************************


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

    #[arg(long, default_value_t = 101, value_parser = clap::value_parser!(usize).range(1..=101))]
    num_points: usize,

    #[arg(long, default_value_t = 2)]
    num_ports: usize,

    #[arg(long)]
    if_bandwidth: Option<u32>,
}

fn main() {

    let args = Args::parse();

    let ports = tokio_serial::available_ports()
        .expect("Failed to enumerate serial ports");

    if ports.is_empty() {
        eprintln!("No VNAs found");
        return;
    }


    // Checks if the serial port is connected
    let vnas_to_use = ports
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

