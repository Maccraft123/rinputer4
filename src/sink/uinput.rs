use crate::{
    sink::Sink,
    source::{OpenedEventSource, SourceCaps},
};
use std::{
    sync::{mpsc, Mutex},
};
use evdev::{
    InputEvent,
};

pub struct UinputSink {
    source_name: String,
    source_caps: SourceCaps,
    //todo
}

fn sink_worker(src: OpenedEventSource) {
    for r in src.chan.recv() {
        println!("{:#?}", r);
    }
}

impl Sink for UinputSink {
    fn name(&self) -> &'static str {
        "Gamepad device"
    }
    fn new(source: OpenedEventSource) -> Box<dyn Sink> {
        let out = Box::new(UinputSink{
            source_name: source.name.clone(),
            source_caps: source.caps,
        });

        std::thread::spawn(|| sink_worker(source));
        out
    }
    fn source_name(&self) -> String {
        self.source_name.clone()
    }
    fn source_caps(&self) -> SourceCaps {
        self.source_caps
    }
}
