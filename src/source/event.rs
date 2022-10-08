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

fn usb_manufacturer_product(input: String) -> Option<String> {
    // input: usb-0000:09:00.3-3/input0
    if let Some(path_with_input) = input.strip_prefix("usb-") {
        // 0000:09:00.3-3/input0
        let path_itself = path_with_input.split('/')
            .map(|v| v.to_string())
            .collect::<Vec<String>>()
            .swap_remove(0);
        
        // path_vec = vec!["0000:09:00.3", 3]
        let path_vec = path_itself.split('-')
            .map(|v| v.to_string())
            .collect::<Vec<String>>();

        if let Some((pci_id, usb_path)) = path_vec.get(0).zip(path_vec.get(1)) {
            let path: PathBuf = ["/sys/bus/pci/devices/", pci_id].iter().collect();
            if !path.exists() {
                return None;
            }

            let usb_bus: u32 = usb_path.parse().unwrap();
            let final_path: PathBuf = [
                "/sys/bus/pci/devices/",
                pci_id,
                &format!("usb{}", usb_bus),
                &format!("{}-{}", usb_bus, usb_path)
            ].iter().collect();

            if !final_path.exists() {
                return None;
            }

            let manufacturer_raw = fs::read_to_string(final_path.join("manufacturer")).unwrap_or_default();
            let product_raw = fs::read_to_string(final_path.join("product")).unwrap_or_default();


            let manufacturer = manufacturer_raw.trim();
            let product = product_raw.trim();

            if product.starts_with(&manufacturer) {
                return Some(product.to_string());
            }

            if manufacturer.is_empty() || product.is_empty() {
                return Some(format!("{}{}", manufacturer, product));
            }

            return Some(format!("{} {}", manufacturer, product));
        }
    }

    None
}

pub struct Evdev {
    device: Device,
    path: PathBuf,
    name: Option<String>,
}

unsafe impl Send for Evdev{}
unsafe impl Sync for Evdev{}

impl Evdev {
    fn new(path: PathBuf, mut device: Device) -> Option<Self> {
        // check for gamepads
        if !device.supported_keys().map_or(false, |k| k.contains(Key::BTN_SOUTH)) {
            return None;
        }

        device.grab().ok()?;
        //fs::remove_file(&path).ok()?;

        let mut name = None;
        if let Some(phys_path) = device.physical_path() {
            name = usb_manufacturer_product(phys_path.to_string());
        }

        Some(Self {
            device,
            path,
            name,
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
        if let Some(n) = self.name.clone() {
            return n;
        }
        if let Some(n) = self.device.name() {
            return n.to_string();
        }
        "Linux event device".to_string()
    }
    fn path(self: &Evdev) -> String {
        self.device.physical_path().unwrap_or("Unknown").to_string()
    }
    fn close(self: Evdev) {

    }
}
