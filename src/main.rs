mod source;
mod sink;

use std::net::{
    TcpListener,
    TcpStream,
    Shutdown,
};
use std::io::{
    self,
    Read,
    BufReader,
    BufRead,
    Write,
};
use std::sync::{
    Mutex,
};

use crate::sink::Sink;

static HELP_TEXT: &[u8] = b"Available commands are:
list_sources: Lists all available sources to bind to a sink
list_sinks: Lists all sinks that can have a source bound
add_sink: Adds a sink
list_sink_types: Lists sink types that can be added with add_sink
bind_source_to_sink: Binds a source to a sink
";

fn handle_client(mut stream: TcpStream, all_sinks_mutex: &Mutex<Vec<Box<dyn Sink>>>) {
    let sources = source::enumerate();
    let sink_types = sink::list_names();

    loop {
        let mut buf = String::new();
        let mut buf_reader = BufReader::new(&mut stream);

        let ret = buf_reader.read_line(&mut buf);
        if ret.is_err() {
            return;
        }

        let args: Vec<&str> = buf.trim().split(' ').collect();

        match args[0] {
            "list_sources" => {
                for (i, device) in sources.iter().enumerate() {
                    let tmp = format!("OK:{}:{}:{:?}", i, device.name(), device.get_capabilities());
                    stream.write_all(tmp.as_str().as_bytes());
                    stream.write_all(b"\n").unwrap();
                }
                stream.write_all(b"END_MULTILINE\n").unwrap();
            },
            "add_sink" => {
                if let Ok(snk_type) = args[1].parse::<usize>() {
                    let mut all_sinks = all_sinks_mutex.lock().unwrap();
                    let (_, new_fn) = sink_types[snk_type];
                    all_sinks.push(new_fn());
                }
                stream.write_all(b"OK\n").unwrap()
            },
            "list_sink_types" => {
                for (i, (name, _)) in sink_types.iter().enumerate() {
                    let tmp = format!("OK:{}:{}", i, name);
                    stream.write_all(tmp.as_str().as_bytes());
                    stream.write_all(b"\n").unwrap();
                }
                stream.write_all(b"END_MULTILINE\n").unwrap();
            },
            "list_sinks" => {
                let all_sinks = all_sinks_mutex.lock().unwrap();
                for (i, sink) in all_sinks.iter().enumerate() {
                    let response = format!("OK:{}:{}\n", i, sink.name());
                    stream.write_all(response.as_str().as_bytes()).unwrap();
                }
                stream.write_all(b"END_MULTILINE\n").unwrap();
            }
            "bind_source_to_sink" => {
                if let (Ok(src), Ok(snk)) = (args[1].parse::<usize>(), args[2].parse::<usize>()) {
                    let all_sinks = all_sinks_mutex.lock().unwrap();
                    let response = format!("OK:{}:{}\n", sources[src].name(), all_sinks[snk].name());
                    stream.write_all(response.as_str().as_bytes()).unwrap();
                } else {
                    stream.write_all(b"ERR:Invalid argument\n").unwrap();
                }
            }
            "help" => stream.write_all(HELP_TEXT).unwrap(),
            _ => stream.write_all(b"ERR:Invalid command\n").unwrap(),
        }

        println!("{:#?}", args);
    }
}

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0")?;

    println!("Listening on {}", listener.local_addr()?);

    let mutex = Mutex::new(Vec::new());

    for stream in listener.incoming() {
        handle_client(stream?, &mutex);
    }

    /*
    let devices = source::enumerate();
    for (i, device) in devices.iter().enumerate() {
        println!("Device {}: {}", i, device.name())
    }
    println!("Which device should be used for 1st gamepad?");

    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap();

    let dev_idx = buf.trim().parse::<usize>().unwrap();

    println!("Using device {}", devices[dev_idx].name());*/

    Ok(())
}
