use evdev::InputEvent;
use std::{
    fmt,
    sync::mpsc,
};

use anyhow::Result;
use crate::source::event::Evdev;

pub mod event;

#[derive(Debug)]
pub enum SourceCaps {
    FullX360,
    DpadAndAB,
}

pub trait EventSource: Send + Sync {
    fn start_ev(&self, output_channel: mpsc::Sender<InputEvent>) -> Result<()>;
    
    fn name(&self) -> String;
    fn path(&self) -> String;
    
    fn close(self);

    fn get_capabilities(&self) -> SourceCaps;
}


pub fn enumerate() -> Vec<Box<dyn EventSource>> {
    let mut ret: Vec<Box<dyn EventSource>> = Vec::new();
    let (mut evdev_devices, evdev_chan) = event::enumerate();
    ret.append(&mut evdev_devices);

    ret
}

impl fmt::Debug for dyn EventSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("EventSource")
            .field("name", &self.name())
            .field("path", &self.path())
            .field("capabilities", &self.get_capabilities())
            .finish()
    }
}
