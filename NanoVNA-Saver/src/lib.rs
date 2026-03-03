use polars::prelude::DataFrame;
use std::thread;
use tokio_serial::SerialPortType;

pub mod sweep;

use sweep::SweepParams;

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
    pub no_print: bool,
}

pub fn run(config: RunConfig) -> Result<DataFrame, String> {
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

    if filtered_ports.is_empty() {
        return Err("No NanoVNA devices detected".into());
    }

    let vnas_to_use = filtered_ports.into_iter().take(config.vna_number);

    let mut handles = Vec::new();

    for (idx, port) in vnas_to_use.enumerate() {
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
            no_print: config.no_print,
        };

        handles.push(thread::spawn(move || sweep::run_on_port(params)));
    }
}
