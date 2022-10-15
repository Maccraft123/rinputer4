use crate::{
    sink::Sink,
    source::{OpenedEventSource, SourceCaps},
};
use evdev::{
    uinput::{
        VirtualDeviceBuilder,
        VirtualDevice,
    },
    UinputAbsSetup,
    AbsInfo,
    Key,
    InputId,
    AbsoluteAxisType,
};
use anyhow::Result;

pub struct UinputSink {
    source_name: String,
    source_caps: SourceCaps,
    //todo
}

static MAX_OUT_ANALOG: i32 = 32767;
static MIN_OUT_ANALOG: i32 = -32768;

static MIN_OUT_HAT: i32 = -1;
static MAX_OUT_HAT: i32 = 1;

static MIN_OUT_TRIG: i32 = 0;
static MAX_OUT_TRIG: i32 = 255;

fn sink_worker(src: OpenedEventSource, mut dst: VirtualDevice) {
    loop {
        match src.chan.recv() {
            Ok(ev) => dst.emit(&[ev]).unwrap(),
            Err(_) => return, // assume we got dropped
        }
    }
}

impl Sink for UinputSink {
    fn name(&self) -> &'static str {
        "Gamepad device"
    }
    fn new(source: OpenedEventSource) -> Result<Box<dyn Sink>> {
        let mut keys = evdev::AttributeSet::<Key>::new();
        keys.insert(Key::BTN_SOUTH);
        keys.insert(Key::BTN_EAST);
        keys.insert(Key::BTN_NORTH);
        keys.insert(Key::BTN_WEST);
        keys.insert(Key::BTN_TL);
        keys.insert(Key::BTN_TR);
        keys.insert(Key::BTN_SELECT);
        keys.insert(Key::BTN_START);
        keys.insert(Key::BTN_MODE);
        keys.insert(Key::BTN_THUMBL);
        keys.insert(Key::BTN_THUMBR);

        let input_id = InputId::new(evdev::BusType::BUS_USB, 0x045e, 0x028e, 0x2137);

        let abs_analogs = AbsInfo::new(0, MIN_OUT_ANALOG, MAX_OUT_ANALOG, 16, 256, 0);
        let abs_x = UinputAbsSetup::new(AbsoluteAxisType::ABS_X, abs_analogs);
        let abs_y = UinputAbsSetup::new(AbsoluteAxisType::ABS_Y, abs_analogs);
        let abs_rx = UinputAbsSetup::new(AbsoluteAxisType::ABS_RX, abs_analogs);
        let abs_ry = UinputAbsSetup::new(AbsoluteAxisType::ABS_RY, abs_analogs);

        let abs_triggers = AbsInfo::new(0, MIN_OUT_TRIG, MAX_OUT_TRIG, 0, 0, 0);
        let abs_z = UinputAbsSetup::new(AbsoluteAxisType::ABS_Z, abs_triggers);
        let abs_rz = UinputAbsSetup::new(AbsoluteAxisType::ABS_RZ, abs_triggers);

        let abs_hat = AbsInfo::new(0, MIN_OUT_HAT, MAX_OUT_HAT, 0, 0, 0);
        let abs_hat_x = UinputAbsSetup::new(AbsoluteAxisType::ABS_HAT0X, abs_hat);
        let abs_hat_y = UinputAbsSetup::new(AbsoluteAxisType::ABS_HAT0Y, abs_hat);

        let uinput_handle = VirtualDeviceBuilder::new().unwrap()
            .name(source.name.as_str().as_bytes())
            .input_id(input_id)
            .with_keys(&keys)?
            .with_absolute_axis(&abs_x)?
            .with_absolute_axis(&abs_y)?
            .with_absolute_axis(&abs_rx)?
            .with_absolute_axis(&abs_ry)?
            .with_absolute_axis(&abs_z)?
            .with_absolute_axis(&abs_rz)?
            .with_absolute_axis(&abs_hat_x)?
            .with_absolute_axis(&abs_hat_y)?
            .build().unwrap();

        // TODO: map abs axis values

        let out = Box::new(UinputSink{
            source_name: source.name.clone(),
            source_caps: source.caps,
        });

        std::thread::spawn(|| sink_worker(source, uinput_handle));
        Ok(out)
    }
    fn source_name(&self) -> String {
        self.source_name.clone()
    }
    fn source_caps(&self) -> SourceCaps {
        self.source_caps
    }
}
