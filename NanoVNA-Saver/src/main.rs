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

    let vnas_to_use = ports.into_iter().take(numb_vnas);

    let mut handles = Vec::new();

    for port in vnas_to_use {
        let port_name = port.port_name.clone();

        let handle = thread::spawn(move || {
            sweep::run_on_port(port_name, num_sweeps);
        });

        handles.push(handle);
    }

    for h in handles {
        h.join().unwrap();
    }
}

