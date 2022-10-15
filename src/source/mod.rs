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
    fn make_tx(&self) -> mpsc::Sender<InputEvent>;
    
    fn name(&self) -> String;
    fn path(&self) -> String;
    
    fn get_capabilities(&self) -> SourceCaps;
}

pub struct OpenedEventSource {
    pub name: String,
    pub path: String,
    pub caps: SourceCaps,
    pub chan: mpsc::Receiver<InputEvent>,
    pub chan_tx: mpsc::Sender<InputEvent>,
}

pub fn into_opened(input: Box<dyn EventSource>) -> OpenedEventSource {
    OpenedEventSource {
        name: input.name(),
        path: input.path(),
        caps: input.get_capabilities(),
        chan_tx: input.make_tx(),
        chan: input.start_ev(),
    }
}

// NOTE: TL + TR matching spans across devices, TL2 + TL doesn't
#[derive(Debug, Clone, Copy)]
enum WaitingEvent {
    TL(bool),
    TR(bool),
    TL2(bool),
    TR2(bool),
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

struct TwoJoycons {
    left: Option<OpenedEventSource>,
    right: Option<OpenedEventSource>,
}

impl TwoJoycons {
    fn have_both(&self) -> bool {
        self.left.is_some() && self.right.is_some()
    }
    fn have_one(&self) -> bool {
        self.left.is_some() || self.right.is_some()
    }
}

// TL from left + TR from right = both
// TR from left + TR2 from left = left
// TL from right + TL2 from right = right

fn actually_wait_joycon(mut maybe_left: Option<OpenedEventSource>, mut maybe_right: Option<OpenedEventSource>, out: mpsc::Sender<OpenedEventSource>) {
    let mut left_tl = false;
    let mut left_tr = false;
    let mut left_tr2 = false;

    let mut right_tr = false;
    let mut right_tl = false;
    let mut right_tl2 = false;

    loop {
        if let Some(ref right) = maybe_right {
            if let Ok(ev) = right.chan.try_recv() {
                match ev.kind() {
                    InputEventKind::Key(key) => {
                        match key {
                            Key::BTN_TR => right_tr = ev.value() != 0,
                            Key::BTN_TL => right_tl = ev.value() != 0,
                            Key::BTN_TL2 => right_tl2 = ev.value() != 0,
                            _ => (),
                        }
                    },
                    _ => (),
                }
            }
        }
        if let Some(ref left) = maybe_left {
            if let Ok(ev) = left.chan.try_recv() {
                match ev.kind() {
                    InputEventKind::Key(key) => {
                        match key {
                            Key::BTN_TL => left_tl = ev.value() != 0,
                            Key::BTN_TR => left_tr = ev.value() != 0,
                            Key::BTN_TR2 => left_tr2 = ev.value() != 0,
                            _ => (),
                        }
                    },
                    _ => (),
                }
            }
        }

        if left_tl && right_tr {
            // combine both devices
            let mut left = maybe_left.unwrap();
            let mut right = maybe_right.unwrap();
            left.caps = SourceCaps::FullX360;
            left.name = String::from("Nintendo Switch Both Joy-Cons");

            let to_left = left.chan_tx.clone();
            std::thread::spawn(move || {
                loop {
                    for ev in right.chan.recv() {
                        if to_left.send(ev).is_err() {
                            return;
                        }
                    }
                }
            });
            
            out.send(left);
            return;
        }
        if left_tr && left_tr2 {
            out.send(maybe_left.unwrap());
            return;
        }
        if right_tl && right_tl2 {
            out.send(maybe_right.unwrap());
            return;
        }
    }
}

pub fn wait_for_lr(input: Vec<OpenedEventSource>) -> OpenedEventSource {
    let (tx, rx) = mpsc::channel();
    let mut joycons = TwoJoycons { left: None, right: None };

    for dev in input {
        let new_tx = tx.clone();
        if dev.name.contains("Joy-Con") {
            if dev.name.contains("Left") {
                joycons.left = Some(dev);
            } else {
                joycons.right = Some(dev);
            }
        } else {
            std::thread::spawn(|| actually_wait(dev, new_tx));
        }

        if joycons.have_both() {
            let new_tx = tx.clone();
            std::thread::spawn(move || actually_wait_joycon(joycons.left.take(), joycons.right.take(), new_tx));
            joycons = TwoJoycons { left: None, right: None }
        }
    }

    if joycons.have_one() {
        let new_tx = tx.clone();
        std::thread::spawn(move || actually_wait_joycon(joycons.left.take(), joycons.right.take(), new_tx));
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
