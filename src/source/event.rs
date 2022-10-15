use evdev::{
    Device,
    InputEvent,
    Key,
    AbsoluteAxisType,
};
use crate::source::{
    EventSource,
    SourceCaps,
    quirks_db::{
        self,
        InputRemap,
    },
};
use std::{
    sync::mpsc::{channel, Sender, Receiver},
    path::{Path, PathBuf},
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

#[derive(Debug)]
enum EvdevQuirks {
    RemapCodes(InputRemap),
    //MergeWithDevice(Device),
    OverrideName(String),
}

fn get_device_quirks(dev: &Device, path: &Path) -> Vec<EvdevQuirks> {
    let mut ret = Vec::new();
    let dmi_quirk = quirks_db::get_dmi_quirk(path);

    if let Some(phys_path) = dev.physical_path() {
        if dmi_quirk.is_some() {
            let quirk = EvdevQuirks::OverrideName("Built-in Controller".to_string());
            ret.push(quirk);
        } else {
            if let Some(name) = usb_manufacturer_product(phys_path.to_string()) {
                let quirk = EvdevQuirks::OverrideName(name);
                ret.push(quirk);
            }
        }
    }

    if let Some(actual_dmi_quirk) = dmi_quirk {
        ret.extend(actual_dmi_quirk.remap_codes.into_iter()
                   .map(|v| EvdevQuirks::RemapCodes(v)));
    }

    if let Some(name) = dev.name() {
        if name.contains("Left Joy-Con") {
            let remaps = vec![
                InputRemap::KeyToAbs(Key::BTN_TL2, AbsoluteAxisType::ABS_Z),
                InputRemap::KeyToKey(Key::BTN_Z, Key::BTN_START),

                InputRemap::KeyToAbs(Key::BTN_DPAD_LEFT, AbsoluteAxisType::ABS_HAT0X),
                InputRemap::KeyToAbs(Key::BTN_DPAD_RIGHT, AbsoluteAxisType::ABS_HAT0X),
                InputRemap::KeyToAbs(Key::BTN_DPAD_DOWN, AbsoluteAxisType::ABS_HAT0Y),
                InputRemap::KeyToAbs(Key::BTN_DPAD_UP, AbsoluteAxisType::ABS_HAT0Y),
            ];
            ret.extend(remaps.into_iter().map(|v| EvdevQuirks::RemapCodes(v)));
        } else {
            let remaps = vec![
                InputRemap::KeyToAbs(Key::BTN_TR2, AbsoluteAxisType::ABS_RZ),
            ];
            ret.extend(remaps.into_iter().map(|v| EvdevQuirks::RemapCodes(v)));
        }
    }

    ret
}

#[allow(dead_code)]
pub struct Evdev {
    device: Device,
    override_name: Option<String>,
    remap_events: Vec<InputRemap>,
    sibling_device: Option<Device>,
    tx: Sender<InputEvent>,
    rx: Option<Receiver<InputEvent>>,
}

unsafe impl Send for Evdev{}
unsafe impl Sync for Evdev{}

impl Drop for Evdev {
    fn drop(&mut self) {
        println!("Ungrabbing device");
        self.device.ungrab().unwrap();
    }
}

impl Evdev {
    fn new(path: PathBuf, mut device: Device) -> Option<Self> {
        // check for gamepads
        if !device.supported_keys().map_or(false, |k| k.contains(Key::BTN_SOUTH)) 
        && !device.supported_keys().map_or(false, |k| k.contains(Key::BTN_THUMBL)) {
            return None;
        }

        if device.input_id().version() == 0x2137 {
            return None;
        }

        device.grab().ok()?;
        //fs::remove_file(&path).ok()?;

        let mut override_name = None;
        let mut remap_events = Vec::new();

        let quirks = get_device_quirks(&device, &path);

        for quirk in quirks {
            match quirk {
                EvdevQuirks::RemapCodes(v)          => remap_events.push(v),
                //EvdevQuirks::MergeWithDevice(_)   => todo!("merging with other input device"),
                EvdevQuirks::OverrideName(new)      => override_name = Some(new),
            };
        }

        let (tx, rx) = channel();
        Some(Self {
            device,
            override_name,
            remap_events,
            sibling_device: None,
            tx,
            rx: Some(rx),
        })
    }
}

pub fn enumerate() -> (Vec<Box<dyn EventSource>>, Receiver<Evdev>) {
    let (_tx, rx) = channel();
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

fn worker(mut dev: Evdev) {
    let raw_dev = &mut dev.device;
    let skip_remap = dev.remap_events.is_empty();
    //let skip_mult = true; // TODO
    loop {
        for ev in raw_dev.fetch_events().unwrap() {
            if !skip_remap {
                if let Some(new) = dev.remap_events.iter().find_map(|v| v.apply_quirk(ev)) {
                    if dev.tx.send(new).is_err() {
                        return;
                    }
                    continue;
                }
            }
            if dev.tx.send(ev).is_err() {
                return;
            }
        }
    }
}

impl EventSource for Evdev {
    fn make_tx(&self) -> Sender<InputEvent> {
        self.tx.clone()
    }
    fn start_ev(mut self: Box<Evdev>) -> Receiver<InputEvent> {
        let rx = self.rx.take();
        std::thread::spawn(|| worker(*self));
        rx.unwrap()
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
