use tokio_serial;
use std::io::Write;
use std::time::Duration;
use std::fs;


fn main(){
    if let Ok(ports) = tokio_serial::available_ports() {
        if let Some(p) = ports.first() {
            println!("Connected to port {}", p.port_name);
            let builder = tokio_serial::new(&p.port_name, 115200).timeout(Duration::from_secs(2));
            let mut port = builder.open().unwrap();
            port.write(b"data 0\r").unwrap();
            let mut buf = [0u8; 2700]; 
            let n = port.read(&mut buf).unwrap();
            let data = format!("Read {n} bytes: {slice:?}", slice = &buf[..n]);
            if let Ok(()) = fs::write("test.txt",data){
                println!("Written values to test.txt")
            }
        }
    }
}

