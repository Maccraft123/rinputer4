mod source;
use std::io;

fn main() {
    let devices = source::enumerate();
    for (i, device) in devices.iter().enumerate() {
        println!("Device {}: {}", i, device.name())
    }
    println!("Which device should be used for 1st gamepad?");

    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap();

    let dev_idx = buf.trim().parse::<usize>().unwrap();

    println!("Using device {}", devices[dev_idx].name());
}
