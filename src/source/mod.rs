use evdev::{
    Key,
    InputEvent,
    InputEventKind,
    AbsoluteAxisType,
    EventType,
};
use std::{
    fmt,
    sync::mpsc,
};

use anyhow::Result;

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

fn actually_wait(dev: OpenedEventSource, out: mpsc::Sender<Option<OpenedEventSource>>) {
    let mut pressed_l = false;
    let mut pressed_r = false;
    loop {
        let recv = dev.chan.recv();
        if out.send(None).is_err() {
            return;
        }

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
            out.send(Some(dev)).unwrap();
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

fn joycon_ev_middleman(dev: OpenedEventSource, out: mpsc::Sender<InputEvent>) -> Result<()> {
    let mut last_hatx = 0;
    let mut last_haty = 0;
    let is_right = dev.name.contains("Right");
    loop {
        for ev in dev.chan.recv() {
            match ev.kind() {
                InputEventKind::Key(key) => {
                    if is_right {
                        match key {
                            Key::BTN_EAST => out.send(InputEvent::new(EventType::KEY, Key::BTN_SOUTH.0, ev.value()))?,
                            Key::BTN_WEST => out.send(InputEvent::new(EventType::KEY, Key::BTN_NORTH.0, ev.value()))?,
                            Key::BTN_SOUTH => out.send(InputEvent::new(EventType::KEY, Key::BTN_WEST.0, ev.value()))?,
                            Key::BTN_NORTH => out.send(InputEvent::new(EventType::KEY, Key::BTN_EAST.0, ev.value()))?,
                            Key::BTN_TL2 => out.send(InputEvent::new(EventType::KEY, Key::BTN_TR.0, ev.value()))?,
                            Key::BTN_TR => continue,
                            Key::BTN_MODE => out.send(InputEvent::new(EventType::KEY, Key::BTN_SELECT.0, ev.value()))?,
                            _ => out.send(ev)?,
                        };
                    } else {
                        match key {
                            Key::BTN_TR => out.send(InputEvent::new(EventType::KEY, Key::BTN_TL.0, ev.value())),
                            Key::BTN_TR2 => out.send(InputEvent::new(EventType::KEY, Key::BTN_TR.0, ev.value())),
                            Key::BTN_TL => continue,
                            _ => out.send(ev),
                        }.unwrap();
                    }
                },
                InputEventKind::AbsAxis(abs) => {
                    match abs {
                        AbsoluteAxisType::ABS_HAT0X => {
                            match ev.value() {
                                1 => out.send(InputEvent::new(EventType::KEY, Key::BTN_NORTH.0, 1))?,
                                0 => {
                                    out.send(InputEvent::new(EventType::KEY, Key::BTN_NORTH.0, 0))?;
                                    out.send(InputEvent::new(EventType::KEY, Key::BTN_SOUTH.0, 0))?;
                                },
                                -1 => out.send(InputEvent::new(EventType::KEY, Key::BTN_SOUTH.0, 1))?,
                                _ => unreachable!("Joycons can't make these events"),
                            };
                        },
                        AbsoluteAxisType::ABS_HAT0Y => {
                            match ev.value() {
                                1 => out.send(InputEvent::new(EventType::KEY, Key::BTN_EAST.0, 1))?,
                                0 => {
                                    out.send(InputEvent::new(EventType::KEY, Key::BTN_EAST.0, 0))?;
                                    out.send(InputEvent::new(EventType::KEY, Key::BTN_WEST.0, 0))?;
                                },
                                -1 => out.send(InputEvent::new(EventType::KEY, Key::BTN_WEST.0, 1))?,
                                _ => unreachable!("Joycons can't make these events"),
                            };
                        },
                        AbsoluteAxisType::ABS_Y | AbsoluteAxisType::ABS_RY
                        | AbsoluteAxisType::ABS_X | AbsoluteAxisType::ABS_RX => {
                            let code: u16;
                            let mut mult: i32;
                            let last: &mut i32;
                            if abs == AbsoluteAxisType::ABS_X || abs == AbsoluteAxisType::ABS_Y {
                                mult = 1;
                            } else {
                                mult = -1;
                            }

                            if abs == AbsoluteAxisType::ABS_RX || abs == AbsoluteAxisType::ABS_X {
                                code = AbsoluteAxisType::ABS_HAT0X.0;
                                last = &mut last_hatx;
                                mult *= -1;
                            } else {
                                code = AbsoluteAxisType::ABS_HAT0Y.0;
                                last = &mut last_haty;
                            }

                            let val = if ev.value() < -10000 {
                                -1 * mult
                            } else if ev.value() > 10000 {
                                1 * mult
                            } else {
                                0
                            };
                            if *last != val {
                                out.send(InputEvent::new(EventType::ABSOLUTE, code, val)).unwrap();
                                *last = val;
                            }
                        }
                        _ => continue,
                    }
                },
                _ => out.send(ev).unwrap(),
            }
        }
    }
}

// TL from left + TR from right = both
// TR from left + TR2 from left = left
// TL from right + TL2 from right = right

fn actually_wait_joycon(maybe_left: Option<OpenedEventSource>, maybe_right: Option<OpenedEventSource>, out: mpsc::Sender<Option<OpenedEventSource>>) {
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
                    InputEventKind::Key(Key::BTN_TL) => left_tl = ev.value() != 0,
                    InputEventKind::Key(Key::BTN_TR) => left_tr = ev.value() != 0,
                    InputEventKind::Key(Key::BTN_TR2) => left_tr2 = ev.value() != 0,
                    _ => (),
                }
            }
        }

        if left_tl && right_tr {
            // combine both devices
            let mut left = maybe_left.unwrap();
            let right = maybe_right.unwrap();

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
            
            out.send(Some(left)).unwrap();
            return;
        }
        if left_tr && left_tr2 {
            let left = maybe_left.unwrap();
            
            let (tx, rx) = mpsc::channel();
            let tx_2 = tx.clone();
            let new_left = OpenedEventSource {
                name: left.name.clone(),
                path: left.path.clone(),
                caps: left.caps,
                chan: rx,
                chan_tx: tx,
            };

            std::thread::spawn(move || joycon_ev_middleman(left, tx_2));

            out.send(Some(new_left)).unwrap();
            return;
        }
        if right_tl && right_tl2 {
            let right = maybe_right.unwrap();
            
            let (tx, rx) = mpsc::channel();
            let tx_2 = tx.clone();
            let new_right = OpenedEventSource {
                name: right.name.clone(),
                path: right.path.clone(),
                caps: right.caps,
                chan: rx,
                chan_tx: tx,
            };

            std::thread::spawn(move || joycon_ev_middleman(right, tx_2));

            out.send(Some(new_right)).unwrap();
            return;
        }
        if out.send(None).is_err() {
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

    let mut recv = None;
    while recv.is_none() {
        recv = rx.recv().unwrap();
    }

    recv.unwrap()
}

pub fn enumerate() -> Vec<Box<dyn EventSource>> {
    let mut ret: Vec<Box<dyn EventSource>> = Vec::new();
    let (mut evdev_devices, _) = event::enumerate();
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
