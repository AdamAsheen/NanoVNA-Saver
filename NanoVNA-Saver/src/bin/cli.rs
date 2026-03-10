use clap::Parser;
use polars::prelude::{CsvWriter, SerWriter};
use std::fs::File;
use std::path::PathBuf;

use nanovna_saver::{RunConfig, run};

fn print_row(row: &str) {
    println!("{}", row);
}

#[derive(Parser, Debug)]
struct CliArgs {
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

fn run_cli(args: CliArgs) {
    let output_path = args.path.unwrap_or_else(|| {
        std::env::current_dir()
            .expect("Failed to get current working directory")
            .join("output.csv")
    });

    let label = args.label.unwrap_or_else(|| "default_label".to_string());

    let num_points = if args.num_points > 101 {
        eprintln!(
            "Requested {} points, but NanoVNA supports a maximum of 101. Defaulting to 101.",
            args.num_points
        );
        101
    } else {
        args.num_points
    };

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
        row_callback: if args.no_print { None } else { Some(print_row) },
    };

    let result = match run(config) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };

    let mut final_df = result.dataframe;

    let sweeps_completed = final_df
        .column("sweep_id")
        .expect("missing sweep_id column")
        .n_unique()
        .expect("failed to count sweeps");

    println!("Completed {} sweeps.", sweeps_completed);
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

fn main() {
    let args = CliArgs::parse();
    run_cli(args);
}
