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
