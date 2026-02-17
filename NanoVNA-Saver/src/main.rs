use std::thread;

mod sweep;

    //********************************************************************************************************
    //********************************************************************************************************
    //******************************************Command Line Format*******************************************
    // *******************************************************************************************************
    // **************cargo run [number_of_sweeps] [vna_number] [start_freq] [end_freq] [num_points]***********
    //*****defaults to 1 sweep, 1 vna, start frequency 50kHz, end frequency 900MHz, number of points 101******
    //********************************************************************************************************
    //************Maximum number of points is 101, if the input number is more it will default to 101*********
    //********************************************************************************************************
    //************************************************Example*************************************************
    //**********************************cargo run 5 1 50_000 900_000_000 101**********************************
    //********************************************************************************************************


fn main() {
    let args: Vec<String> = std::env::args().collect();

    let num_sweeps = args.get(1)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);

    let vna_number = args.get(2)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);

    let start_freq: u64 = args.get(3)
        .unwrap_or(&"50_000".to_string())
        .replace('_', "")
        .parse()
        .unwrap();
    let end_freq: u64 = args.get(4)
        .unwrap_or(&"900_000_000".to_string())
        .replace('_', "")
        .parse()
        .unwrap();
    let num_points: usize = args.get(5)
        .unwrap_or(&"101".to_string())
        .replace('_', "")
        .parse()
        .unwrap_or(101)
        .min(101);

    let ports = tokio_serial::available_ports()
        .expect("Failed to enumerate serial ports");

    if ports.is_empty() {
        eprintln!("No VNAs found");
        return;
    }


    // Checks if the serial port is connected
    let vnas_to_use = ports.into_iter().take(vna_number);

    // Print line for table header
    println!("| ID | Label | VNA NUMBER | TIME COMMAND SENT | TIME READING RECEIVED | Frequency | SParameter | Real | Imaginary |");

    let mut handles = Vec::new();

    for (idx, port) in vnas_to_use.enumerate() {
        let port_name = port.port_name.clone();
        let vna_number = idx + 1; 

        let handle = thread::spawn(move || {
            sweep::run_on_port(port_name, num_sweeps, vna_number, start_freq, end_freq, num_points);
        });

        handles.push(handle);
    }

    for h in handles {
        h.join().unwrap();
    }
}

