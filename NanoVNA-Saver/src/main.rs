use tokio_serial;
use tokio_serial::SerialPortBuilderExt;
use std::io::Write;
use std::time::Duration;

fn main(){
    if let Ok(ports) = tokio_serial::available_ports() {
        if let Some(p) = ports.first() {
            println!("{}", p.port_name);
            let builder = tokio_serial::new(&p.port_name, 115200).timeout(Duration::from_secs(2));
            let mut port = builder.open().unwrap();
            port.write(b"sweep\r").unwrap();
            let mut buf = [0u8; 1024]; 
            let n = port.read(&mut buf).unwrap();
            println!("Read {} bytes: {:?}", n, &buf[..n]);
        }
    }
}

