use polars::prelude::DataFrame;
use std::thread;
use tokio_serial::SerialPortType;

pub mod sweep;

use swee::SweepParams;
