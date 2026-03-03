use clap::Parser;
use polars::prelude::{CsvWriter, SerWriter};
use std::fs::File;
use std::path::PathBuf;
use std::thread;
use tokio_serial::SerialPortType;
mod gui;
mod sweep;
use gui::NanoVNASaverApp;

#[derive(Parser, Debug)]
#[command(name = "nanovna-saver")]
#[command(about = "Performs NanoVNA sweeps with configurable parameters")]
struct Args {
    #[arg(short = 's', long, default_value_t = 1, conflicts_with = "time")]
    num_sweeps: usize,

    #[arg(short = 'd', long, default_value_t = 1)]
    vna_number: usize,

    #[arg(long, default_value_t = 50_000)]
    start_freq: u64,

    #[arg(long, default_value_t = 900_000_000)]
    end_freq: u64,

    #[arg(short = 'p', long, default_value_t = 101)]
    num_points: usize,

    #[arg(long, default_value_t = 2)]
    num_ports: usize,

    #[arg(short = 'i', long)]
    if_bandwidth: Option<u32>,

    #[arg(long)]
    path: Option<PathBuf>,

    #[arg(long, conflicts_with = "num_sweeps")]
    time: Option<u64>,

    #[arg(long)]
    label: Option<String>,

    #[arg(long)]
    no_save: bool,

    #[arg(long)]
    no_print: bool,
}

fn main() {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "NanoVNA-Saver",
        options,
        Box::new(|_cc| Box::new(NanoVNASaverApp::default())),
    );

    let args = Args::parse();

    let output_path = args.path.unwrap_or_else(|| {
        std::env::current_dir()
            .expect("Failed to get current working directory")
            .join("output.csv")
    });

    let label = args.label.unwrap_or_else(|| "default_label".to_string());

    let Args {
        num_sweeps,
        vna_number,
        start_freq,
        end_freq,
        mut num_points,
        num_ports,
        if_bandwidth,
        time,
        no_save,
        no_print,
        ..
    } = args;

    // Limit num_points to 101 if more are typed
    if num_points > 101 {
        println!("num_points limited to 101 (was {})", num_points);
        num_points = 101;
    }

    let ports = tokio_serial::available_ports().expect("Failed to enumerate serial ports");

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

    let vnas_to_use = filtered_ports.into_iter().take(vna_number);

    // Print line for table header
    if !no_print {
        println!(
            "| ID | Label | VNA NUMBER | TIME COMMAND SENT | TIME READING RECEIVED | Frequency | SParameter | Real | Imaginary |"
        );
    }

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
            time,
            label: label.clone(),
            no_print,
        };
        let handle = thread::spawn(move || sweep::run_on_port(params));

        handles.push(handle);
    }

    let mut dataframes = Vec::new();

    for h in handles {
        let df = h.join().expect("Thread panicked").expect("Sweep failed");
        dataframes.push(df);
    }

    let mut iter = dataframes.into_iter();
    let mut final_df = iter.next().expect("No data collected");

    for df in iter {
        final_df
            .vstack_mut(&df)
            .expect("Failed to stack DataFrames");
    }

    if !no_save {
        let mut file = File::create(&output_path).expect("Failed to create CSV file");

        CsvWriter::new(&mut file)
            .include_header(true)
            .finish(&mut final_df)
            .expect("Failed to write CSV");

        println!("Saved CSV to {:?}", output_path);
    }
}
