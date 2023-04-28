fn main() {
    println!("Agon VDP SDL started.");

    let ports = serialport::available_ports().expect("No ports found!");
    for p in ports {
        println!("{}", p.port_name);
    }

    use core::time::Duration;

    let mut port = serialport::new("/dev/ttyUSB0", 1152_000)
    .timeout(Duration::from_millis(10))
    .open().expect("Failed to open port");

    let mut serial_buf: Vec<u8> = vec![0; 1];

    println!("Starting read loop.");
    loop
    {
        match port.read(serial_buf.as_mut_slice())
        {
            Ok(n) =>
            {
                //println!("Received: {:#02x}", serial_buf[0]);
                match serial_buf[0] {
                    0x08 => println!("Cursor Left"),
                    n @ _ => println!("Unknown Command {:#02x} received!", n),
                }
            },
            Err(e) => (), //println!("Error: {}", e),
        }
    }

    println!("Bye!");
}
