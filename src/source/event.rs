use evdev::{
    Device,
    InputEvent,
    Key,
    AbsoluteAxisType,
};
use crate::source::{
    EventSource,
    SourceCaps,
    quirks_db,
};
use anyhow::Result;
use std::{
    sync::mpsc::{channel, Sender, Receiver},
    path::PathBuf,
    collections::HashMap,
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

fn get_dmi(name: &str) -> String {
    let path = format!("/sys/class/dmi/id/{}", name);
    match std::fs::read_to_string(&path) {
        Ok(s) => s.lines().next().unwrap_or("<failed to read>").to_string(),
        Err(_) => "<failed to read>".to_string()
    }
}

fn match_str(inp: &str, x: &str, relaxed: bool) -> bool {
    if inp.is_empty() {
        true
    } else {
        if relaxed {
            inp.contains(x) || x.contains(inp)
        } else {
            inp == x
        }
    }
}

fn internal_controller_name(phys_path: &str) -> Option<String> {
    #[cfg(target_arch = "x86_64")]
    {
        let dmi_names = quirks_db::dmi();

        let product_name = get_dmi("product_name");
        let product_vendor = get_dmi("product_vendor");
        let board_name = get_dmi("board_name");
        let board_vendor = get_dmi("board_vendor");

        for dev in dmi_names {
            let pn_match = match_str(&dev.product_name, &product_name, dev.relaxed_name);
            let pv_match = match_str(&dev.product_vendor, &product_vendor, dev.relaxed_vendor);
            let bn_match = match_str(&dev.board_name, &board_name, dev.relaxed_name);
            let bv_match = match_str(&dev.board_vendor, &board_vendor, dev.relaxed_vendor);
            if pn_match && pv_match && bn_match && bv_match {
                if phys_path.contains(dev.phys_path) {
                    return Some("Internal controller".to_string());
                } else {
                    eprintln!("Found matches for all dmi strings, phys path didn't match");
                    eprintln!("Got: '{}'", phys_path);
                    eprintln!("Expected '{}'", dev.phys_path);
                }
            }
        }
    }
    None
}

#[derive(Debug)]
enum EvdevQuirks {
    RemapCodes{from: u16, to: u16},
    //MergeWithDevice(Device),
    OverrideName(String),
}

fn get_device_quirks(dev: &Device) -> Vec<EvdevQuirks> {
    let mut ret = Vec::new();

    if let Some(phys_path) = dev.physical_path() {
        if let Some(name) = internal_controller_name(&phys_path) {
            let quirk = EvdevQuirks::OverrideName(name);
            ret.push(quirk);
        } else {
            if let Some(name) = usb_manufacturer_product(phys_path.to_string()) {
                let quirk = EvdevQuirks::OverrideName(name);
                ret.push(quirk);
            }
        }
    }

    ret
}

pub struct Evdev {
    device: Device,
    path: PathBuf,
    override_name: Option<String>,
    remap_events: Option<HashMap<u16, u16>>,
    sibling_device: Option<Device>,
}

unsafe impl Send for Evdev{}
unsafe impl Sync for Evdev{}

impl Evdev {
    fn new(path: PathBuf, mut device: Device) -> Option<Self> {
        // check for gamepads
        if !device.supported_keys().map_or(false, |k| k.contains(Key::BTN_SOUTH)) 
        && !device.supported_keys().map_or(false, |k| k.contains(Key::BTN_THUMBL)) {
            return None;
        }

        device.grab().ok()?;
        //fs::remove_file(&path).ok()?;

        let mut override_name = None;
        let mut remap_events = HashMap::new();

        let quirks = get_device_quirks(&device);
        for quirk in quirks {
            match quirk {
                EvdevQuirks::RemapCodes{from, to}   => { remap_events.insert(from, to); },
                //EvdevQuirks::MergeWithDevice(_)     => todo!("merging with other input device"),
                EvdevQuirks::OverrideName(new)      => override_name = Some(new),
            };
        }

        Some(Self {
            device,
            path,
            override_name,
            remap_events: if remap_events.is_empty() { None } else { Some(remap_events) },
            sibling_device: None,
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

fn worker(mut dev: Evdev, out: Sender<InputEvent>) {
    let raw_dev = &mut dev.device;
    loop {
        for ev in raw_dev.fetch_events().unwrap() {
            let ret = out.send(ev);
            if ret.is_err() {
                break;
            }
        }
    }
}

impl EventSource for Evdev {
    fn start_ev(self: Evdev, output_channel: Sender<InputEvent>) {
        std::thread::spawn(|| worker(self, output_channel));
    }
    fn name(self: &Evdev) -> String {
        if let Some(n) = self.override_name.clone() {
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
    fn get_capabilities(&self) -> SourceCaps {
        if let Some(keys) = self.device.supported_keys() {
            if keys.contains(Key::BTN_SOUTH) {
                if let Some(axes) = self.device.supported_absolute_axes() {
                    if axes.contains(AbsoluteAxisType::ABS_X) && axes.contains(AbsoluteAxisType::ABS_Y) {
                        SourceCaps::FullX360
                    } else {
                        SourceCaps::DpadAndAB
                    }
                } else {
                    SourceCaps::DpadAndAB
                }
            } else {
                SourceCaps::DpadAndAB
            }
        } else {
            SourceCaps::FullX360
        }
    }
}
