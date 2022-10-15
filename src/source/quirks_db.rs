use evdev::{
    InputEvent,
    Key,
    AbsoluteAxisType,
    EventType,
    InputEventKind,
};

#[derive(Clone, Debug)]
pub struct DmiQuirk {
    pub board_vendor: &'static str,
    pub board_name: &'static str,
    pub product_vendor: &'static str,
    pub product_name: &'static str,
    pub relaxed_name: bool,
    pub relaxed_vendor: bool,
    pub phys_path: &'static str,
    pub remap_codes: Vec<InputRemap>, 
}

#[derive(Copy, Clone, Debug)]
pub enum InputRemap {
    KeyToKey(Key, Key),
    KeyToAbs(Key, AbsoluteAxisType),
    KeyToQuickAccessMenu(Key),
}

impl InputRemap {
    pub fn apply_quirk(self, input: InputEvent) -> Option<InputEvent> {
        if let InputEventKind::Key(input_key) = input.kind() {
            match self {
                InputRemap::KeyToKey(my_key, output_key) => {
                    return Some(InputEvent::new(EventType::KEY, input.code(), input.value()));
                },
                InputRemap::KeyToAbs(my_key, abs) => todo!("Mapping to absolute axis"),
                InputRemap::KeyToQuickAccessMenu(my_key) => todo!("steam quick access menu"),
                _ => return None,
            }
        }
        unreachable!("Should've returned before");
    }
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

pub fn get_dmi_quirk() -> Option<DmiQuirk> {
    let quirks_vec = vec![
        DmiQuirk {
            board_vendor: "AYANEO",
            board_name: "AIR",
            product_vendor: "",
            product_name: "",
            relaxed_name: true,
            relaxed_vendor: false,
            remap_codes: vec![
                InputRemap::KeyToKey(Key::KEY_F12, Key::BTN_MODE),
                InputRemap::KeyToQuickAccessMenu(Key::KEY_D),
            ],
            phys_path: "", // TODO
        },
        DmiQuirk {
            board_vendor: "AYANEO",
            board_name: "NEXT",
            product_vendor: "",
            product_name: "",
            relaxed_name: true,
            relaxed_vendor: false,
            remap_codes: vec![
                InputRemap::KeyToKey(Key::KEY_F12, Key::BTN_MODE),
                InputRemap::KeyToQuickAccessMenu(Key::KEY_D),
            ],
            phys_path: "", // TODO
        }
    ];

    let product_name = get_dmi("product_name");
    let product_vendor = get_dmi("product_vendor");
    let board_name = get_dmi("board_name");
    let board_vendor = get_dmi("board_vendor");

    for quirk in quirks_vec.into_iter() {
        let pn_match = match_str(&quirk.product_name, &product_name, quirk.relaxed_name);
        let pv_match = match_str(&quirk.product_vendor, &product_vendor, quirk.relaxed_vendor);
        let bn_match = match_str(&quirk.board_name, &board_name, quirk.relaxed_name);
        let bv_match = match_str(&quirk.board_vendor, &board_vendor, quirk.relaxed_vendor);
        if pn_match && pv_match && bn_match && bv_match {
            return Some(quirk);
        }
    }

    None
}
