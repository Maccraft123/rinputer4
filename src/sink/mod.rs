use evdev::InputEvent;
use std::{
    fmt,
    sync::mpsc,
};
use crate::{OpenedEventSource, source::SourceCaps};

use anyhow::Result;

pub mod uinput;
use uinput::UinputSink;

pub trait Sink: Send + Sync {
    fn name(&self) -> &str;
    fn new(source: OpenedEventSource) -> Result<Box<dyn Sink>> where Self: Sized;
    fn source_name(&self) -> String;
    fn source_caps(&self) -> SourceCaps;
}

pub fn list_names() -> Vec<(String, fn(OpenedEventSource) -> Result<Box<dyn Sink>>)> {
    vec![
        ("Gamepad device".to_string(), UinputSink::new),
    ]
}
