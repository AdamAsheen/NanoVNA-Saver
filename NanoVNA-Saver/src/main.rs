use clap::Parser;
use polars::prelude::{CsvWriter, SerWriter};
use std::fs::File;
use std::path::PathBuf;

use nanovna_saver::{RunConfig, run};

#[derive(Parser, Debug)]
#[command(name = "nanovna-saver")]
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
    let args = Args::parse();

    let output_path = args.path.unwrap_or_else(|| {
        std::env::current_dir()
            .expect("Failed to get current working directory")
            .join("output.csv")
    });

    let label = args.label.unwrap_or_else(|| "default_label".to_string());

    let num_points = args.num_points.min(101);

    let config = RunConfig {
        num_sweeps: args.num_sweeps,
        vna_number: args.vna_number,
        start_freq: args.start_freq,
        end_freq: args.end_freq,
        num_points,
        num_ports: args.num_ports,
        if_bandwidth: args.if_bandwidth,
        time: args.time,
        label,
        no_print: args.no_print,
    };

    let result = match run(config) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };

    let mut final_df = result.dataframe;

    if !args.no_print {
        println!(
            "| ID | Label | VNA NUMBER | TIME COMMAND SENT | TIME READING RECEIVED | Frequency | SParameter | Real | Imaginary |"
        );

        let sweep_id = final_df.column("sweep_id").unwrap().str().unwrap();
        let label = final_df.column("label").unwrap().str().unwrap();
        let vna_number = final_df.column("vna_number").unwrap().i32().unwrap();
        let time_cmd_sent = final_df.column("time_cmd_sent").unwrap().f64().unwrap();
        let time_received = final_df.column("time_received").unwrap().f64().unwrap();
        let freq = final_df.column("frequency_hz").unwrap().f64().unwrap();
        let channel = final_df.column("channel").unwrap().str().unwrap();
        let real = final_df.column("real").unwrap().f64().unwrap();
        let imag = final_df.column("imag").unwrap().f64().unwrap();

        for i in 0..final_df.height() {
            println!(
                "| {} | {} | {} | {:.6} | {:.6} | {:.0} | {} | {} | {} |",
                sweep_id.get(i).unwrap(),
                label.get(i).unwrap(),
                vna_number.get(i).unwrap(),
                time_cmd_sent.get(i).unwrap(),
                time_received.get(i).unwrap(),
                freq.get(i).unwrap(),
                channel.get(i).unwrap(),
                real.get(i).unwrap(),
                imag.get(i).unwrap()
            );
        }
    }

    println!("Total bytes read: {}", result.total_bytes);
    println!("Elapsed time: {:.2} s", result.elapsed_seconds);

    if !args.no_save {
        let mut file = File::create(&output_path).expect("Failed to create CSV file");

        CsvWriter::new(&mut file)
            .include_header(true)
            .finish(&mut final_df)
            .expect("Failed to write CSV");

        println!("Saved CSV to {:?}", output_path);
    }
}
