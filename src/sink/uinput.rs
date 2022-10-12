use crate::sink::Sink;
use std::{
    sync::mpsc,
};
use evdev::{
    InputEvent,
};

pub struct UinputSink {

    //todo
}

impl Sink for UinputSink {
    fn add_chan(&mut self, chan: mpsc::Receiver<InputEvent>) {
        todo!("adding input chans for uinput sinks");
    }
    fn name(&self) -> String {
        "Userspace Input Device".to_string()
    }
    fn new() -> Box<dyn Sink> {
        Box::new(UinputSink{})
    }
}
