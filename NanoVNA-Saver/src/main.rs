use std::thread;

mod sweep;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let num_sweeps = args.get(1)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);

    let numb_vnas = args.get(2)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);

    let ports = tokio_serial::available_ports()
        .expect("Failed to enumerate serial ports");

    if ports.is_empty() {
        eprintln!("No VNAs found");
        return;
    }
    // Checks if the serial port is connected
    let vnas_to_use = ports
    .into_iter()
    .filter(|p|{
        let n = p.port_name.as_str();
        n.starts_with("/dev/cu.") && n.contains("usbmodem")
    })
    .take(numb_vnas);

    let mut handles = Vec::new();

    // Print line for table header
    println!("| ID | Label | VNA NUMBER | TIME COMMAND SENT | TIME READING RECEIVED | Frequency | SParameter | Real | Imaginary |");

    for (idx, port) in vnas_to_use.enumerate() {
        let port_name = port.port_name.clone();
        let vna_number = idx + 1; 

        let handle = thread::spawn(move || {
            sweep::run_on_port(port_name, vna_number, num_sweeps);
        });

        handles.push(handle);
    }

    for h in handles {
        h.join().unwrap();
    }
}

