use evdev::{
    Device,
    InputEvent,
    Key
};
use crate::source::EventSource;
use anyhow::Result;
use std::{
    sync::mpsc::{channel, Sender, Receiver},
    path::PathBuf,
    fs,
};

pub struct Evdev {
    my_device: Device,
    my_path: PathBuf,
}

unsafe impl Send for Evdev{}
unsafe impl Sync for Evdev{}

impl Evdev {
    fn new(path: PathBuf, mut my_device: Device) -> Option<Self> {
        // check for gamepads
        if !my_device.supported_keys().map_or(false, |k| k.contains(Key::BTN_SOUTH)) {
            return None;
        }

        my_device.grab().ok()?;
        fs::remove_file(&path).ok()?;

        Some(Self {
            my_device,
            my_path: path,
        })
    }
}

pub fn enumerate() -> (Vec<Box<dyn EventSource>>, Receiver<Evdev>) {
    let (tx, rx) = channel();
    let tmp: Vec<Evdev> = evdev::enumerate()
        .filter_map(|(p, d)| Evdev::new(p, d))
        .collect();

    let mut ret = Vec::new();
    for device in tmp {
        let tmp_box: Box<dyn EventSource> = Box::new(device);
        ret.push(tmp_box);
    };

    (ret, rx)
}

impl EventSource for Evdev {
    fn start_ev(self: &Evdev, output_channel: Sender<InputEvent>) -> Result<()> {
        Ok(())
    }
    fn name(self: &Evdev) -> String {
        self.my_device.name().unwrap_or("Linux event device").to_string()
    }
    fn path(self: &Evdev) -> String {
        self.my_device.physical_path().unwrap_or("Unknown").to_string()
    }
    fn close(self: Evdev) {

    }
}
