use evdev::InputEvent;
use std::{
    fmt,
    sync::mpsc,
};

use anyhow::Result;

pub mod uinput;
use uinput::UinputSink;

pub trait Sink: Send + Sync {
    fn new() -> Box<dyn Sink> where Self: Sized;
    fn add_chan(&mut self, input: mpsc::Receiver<InputEvent>);
    fn name(&self) -> String;
}

pub fn list_names() -> Vec<(String, fn() -> Box<dyn Sink>)> {

    vec![
        (UinputSink::new().name(), UinputSink::new),
    ]
}
