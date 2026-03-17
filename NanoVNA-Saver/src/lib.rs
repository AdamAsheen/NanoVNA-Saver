use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread;
use tokio_serial::SerialPortInfo;
use tokio_serial::SerialPortType;

pub mod gui;
pub mod sweep;

use sweep::{SweepParams, SweepResult};

pub struct RunConfig {
    pub num_sweeps: usize,
    pub vna_number: usize,
    pub start_freq: u64,
    pub end_freq: u64,
    pub num_points: usize,
    pub num_ports: usize,
    pub if_bandwidth: Option<u32>,
    pub time: Option<u64>,
    pub label: String,
    pub row_callback: Option<fn(&str)>,
    pub stop_flag: Arc<AtomicBool>,
}

fn get_filtered_nanovna_ports() -> Result<Vec<SerialPortInfo>, String> {
    let ports = tokio_serial::available_ports().map_err(|_| "Failed to enumerate serial ports")?;

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

    Ok(filtered_ports)
}

pub fn detect_nanovna_port_names() -> Result<Vec<String>, String> {
    Ok(get_filtered_nanovna_ports()?
        .into_iter()
        .map(|p| p.port_name)
        .collect())
}

pub fn run(config: RunConfig) -> Result<SweepResult, String> {
    let filtered_ports = get_filtered_nanovna_ports()?;

    if filtered_ports.is_empty() {
        return Err("No NanoVNA devices detected".into());
    }

    let vnas_to_use = filtered_ports.into_iter().take(config.vna_number);

    let mut handles = Vec::new();

    for (idx, port) in vnas_to_use.enumerate() {
        println!("Connected to NanoVNA {} on {}", idx + 1, port.port_name);

        let params = SweepParams {
            port_name: port.port_name.clone(),
            num_sweeps: config.num_sweeps,
            vna_number: idx + 1,
            start_freq: config.start_freq,
            end_freq: config.end_freq,
            num_points: config.num_points,
            num_ports: config.num_ports,
            if_bandwidth: config.if_bandwidth,
            time: config.time,
            label: config.label.clone(),
            row_callback: config.row_callback,
            stop_flag: Arc::clone(&config.stop_flag),
        };

        handles.push(thread::spawn(move || sweep::run_on_port(params)));
    }
    let mut dataframes = Vec::new();
    let mut total_bytes = 0;
    let mut total_time = 0.0;

    for h in handles {
        let result = h
            .join()
            .map_err(|_| "Thread panicked".to_string())?
            .map_err(|e| format!("Sweep failed: {e}"))?;

        total_bytes += result.total_bytes;
        total_time += result.elapsed_seconds;

        dataframes.push(result.dataframe);
    }

    let mut iter = dataframes.into_iter();
    let mut final_df = iter.next().ok_or("No data collected")?;

    for df in iter {
        final_df
            .vstack_mut(&df)
            .map_err(|_| "Failed to stack DataFrames")?;
    }

    Ok(SweepResult {
        dataframe: final_df,
        total_bytes,
        elapsed_seconds: total_time,
    })
}
