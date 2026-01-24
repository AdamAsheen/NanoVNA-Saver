use tokio_serial;
use std::io::Write;
use std::time::Duration;
use std::fs;
mod sweep;


fn main(){
 let args: Vec<String> = std::env::args().collect();
 let num_sweeps = args.get(1)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);
sweep::run(num_sweeps);
}

