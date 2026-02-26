use std::thread;
use clap::Parser;
use std::path::PathBuf;
use polars::frame::DataFrame;
use polars::prelude::CsvWriter;
use std::fs::File;
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

    #[arg(long)]
    path: Option<PathBuf>,

}

fn main() {

    let args = Args::parse();

    let output_path = args.path.unwrap_or_else(|| {
    std::env::current_dir()
        .expect("Failed to get current working directory").join("output.csv")
    });

    let Args {
    num_sweeps,
    vna_number,
    start_freq,
    end_freq,
    mut num_points,
    num_ports,
    if_bandwidth,
    ..
    } = args;

        // Limit num_points to 101 if more are typed
    if num_points > 101 {
        println!("num_points limited to 101 (was {})", num_points);
        num_points = 101;
    }

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
            sweep::run_on_port(params)
        });

        handles.push(handle);
    }

    let mut dataframes = Vec::new();

    for h in handles {
        let df = h.join().expect("Thread panicked");
        dataframes.push(df);
    }

    let mut iter = dataframes.into_iter();
    let mut final_df = iter.next().expect("No data collected");

    for df in iter {
        final_df.vstack_mut(&df).expect("Failed to stack DataFrames");
}
}

