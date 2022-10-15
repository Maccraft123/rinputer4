use evdev::{
    Key,
    InputEvent,
    InputEventKind,
};
use std::{
    fmt,
    sync::mpsc,
};

use anyhow::Result;
use crate::source::event::Evdev;

mod quirks_db;
pub mod event;

#[derive(Debug, Copy, Clone)]
pub enum SourceCaps {
    FullX360,
    DpadAndAB,
}

pub trait EventSource: Send + Sync {
    fn start_ev(self: Box<Self>) -> mpsc::Receiver<InputEvent>;
    
    fn name(&self) -> String;
    fn path(&self) -> String;
    
    fn get_capabilities(&self) -> SourceCaps;
}

pub struct OpenedEventSource {
    pub name: String,
    pub path: String,
    pub caps: SourceCaps,
    pub chan: mpsc::Receiver<InputEvent>,
}

pub fn into_opened(input: Box<dyn EventSource>) -> OpenedEventSource {
    OpenedEventSource {
        name: input.name(),
        path: input.path(),
        caps: input.get_capabilities(),
        chan: input.start_ev(),
    }
}

fn actually_wait(dev: OpenedEventSource, out: mpsc::Sender<OpenedEventSource>) {
    let mut pressed_l = false;
    let mut pressed_r = false;
    loop {
        let recv = dev.chan.recv();
        if let Ok(ev) = recv {
            match ev.kind() {
                InputEventKind::Key(Key::BTN_TR) => pressed_r = if ev.value() == 1 { true } else { false },
                InputEventKind::Key(Key::BTN_TL) => pressed_l = if ev.value() == 1 { true } else { false },
                _ => (),
            }
        }

        if recv.is_err() {
            return;
        }

        if pressed_l && pressed_r {
            out.send(dev);
            return;
        }
    }
}

pub fn wait_for_lr(input: Vec<OpenedEventSource>) -> OpenedEventSource {
    let (tx, rx) = mpsc::channel();

    for dev in input {
        let new_tx = tx.clone();
        std::thread::spawn(|| actually_wait(dev, new_tx));
    }

    rx.recv().unwrap()
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
