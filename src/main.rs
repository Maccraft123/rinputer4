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
    Arc,
    Mutex,
};

use crate::{
    sink::Sink,
    source::OpenedEventSource,
};

static HELP_TEXT: &[u8] = b"Available commands are:
list_sinks: Lists all sinks in use with sources attached to them
add_sink: Adds a sink and autobinds a source
list_sink_types: Lists sink types that can be added with add_sink
help: Displays this message
";

fn handle_client(mut stream: TcpStream, all_sinks_mutex: Arc<Mutex<Vec<Box<dyn Sink>>>>) {
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
            "add_sink" => {
                if let Ok(snk_type) = args[1].parse::<usize>() {
                    let mut all_sinks = all_sinks_mutex.lock().unwrap();
                    let (_, new_fn) = sink_types[snk_type];

                    let cur_sources = source::enumerate().into_iter()
                        .map(|v| source::into_opened(v))
                        .collect::<Vec<OpenedEventSource>>();
                    let new_source = source::wait_for_lr(cur_sources);


                    all_sinks.push(new_fn(new_source));
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
                    let response = format!("OK:{}:{}:{}\n", i, sink.name(), sink.source_name());
                    stream.write_all(response.as_str().as_bytes()).unwrap();
                }
                stream.write_all(b"END_MULTILINE\n").unwrap();
            }
            "help" => stream.write_all(HELP_TEXT).unwrap(),
            _ => stream.write_all(b"ERR:Invalid command\n").unwrap(),
        }

        println!("{:#?}", args);
    }
}

fn main() -> io::Result<()> {


    let mutex = Arc::new(Mutex::new(Vec::new()));
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    println!("Listening on {}", listener.local_addr().unwrap());
    for stream in listener.incoming() {
        let ptr = Arc::clone(&mutex);
        std::thread::spawn(move || handle_client(stream.unwrap(), ptr));
    };

    Ok(())
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
}
