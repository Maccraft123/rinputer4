pub struct DmiQuirk {
    pub board_vendor: &'static str,
    pub board_name: &'static str,
    pub product_vendor: &'static str,
    pub product_name: &'static str,
    pub relaxed_name: bool,
    pub relaxed_vendor: bool,
    pub phys_path: &'static str,
}

pub fn dmi() -> Vec<DmiQuirk> {
    vec![
        DmiQuirk {
            board_vendor: "AYANEO",
            board_name: "AIR",
            product_vendor: "",
            product_name: "",
            relaxed_name: true,
            relaxed_vendor: false,
            phys_path: "", // TODO
        },
        DmiQuirk {
            board_vendor: "AYANEO",
            board_name: "NEXT",
            product_vendor: "",
            product_name: "",
            relaxed_name: true,
            relaxed_vendor: false,
            phys_path: "", // TODO
        }
    ]
}
