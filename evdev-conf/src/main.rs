use std::ops::Index;

mod octant;
mod map;

use octant::OctantSection;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct LR<T> {
    pub l: T,
    pub r: T,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct XY {
    pub x: i16,
    pub y: i16,
}

impl XY {
    pub fn x_f64(self) -> f64 {
        self.x as f64 / 32767.0
    }

    pub fn y_f64(self) -> f64 {
        self.y as f64 / 32767.0
    }

    pub fn octant(self) -> Option<octant::OctantSection> {
        octant::polar_to_octant(octant::xy_to_vel_cir(self.x_f64(), self.y_f64()))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct Buttons {
    n: bool,
    e: bool,
    s: bool,
    w: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct MyState {
    pub sticks: LR<XY>,
    pub buttons: LR<Buttons>,
    pub l1: bool,
    pub l2: bool,
    pub r1: bool,
    pub r2: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct HalfState {
    pub stick: XY,
    pub buttons: Buttons,
}

impl MyState {
    pub fn get_primary_half(self) -> HalfState {
        HalfState { stick: self.sticks.l, buttons: self.buttons.r }
    }

    pub fn get_secondary_half(self) -> HalfState {
        HalfState { stick: self.sticks.r, buttons: self.buttons.l }
    }
}


#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct Transition<T> {
    pub prev: T,
    pub curr: T,
}

impl<T> Transition<T> {
    pub fn push_new(&mut self, new: T) {
        std::mem::swap(&mut self.prev, &mut self.curr);
        self.curr = new;
    }

    pub fn change<F: FnMut(&T) -> bool>(&self, mut f: F) -> Option<bool> {
        let before = f(&self.prev);
        let now = f(&self.curr);
        match (before, now) {
            (false, true)  => Some(true),
            (true, false)  => Some(false),
            (true, true)   => None,
            (false, false) => None,
        }
    }

    pub fn map<U, F: FnMut(T) -> U>(self, mut f: F) -> Transition<U> {
        Transition { prev: f(self.prev), curr: f(self.curr) }
    }

    pub fn as_ref(&self) -> Transition<&T> {
        Transition { prev: &self.prev, curr: &self.curr }
    }
}

impl<T: Eq> Transition<T> {
    pub fn changed(&self) -> bool {
        self.prev != self.curr
    }
}

impl Index<usize> for Buttons {
    type Output = bool;

    fn index(&self, i: usize) -> &bool {
        match i {
            0 => &self.n,
            1 => &self.e,
            2 => &self.s,
            3 => &self.w,
            _ => panic!(),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Report {
    InputData                  = 0x09,
    SetMappings                = 0x80,
    ClearMappings              = 0x81,
    GetMappings                = 0x82,
    GetAttrib                  = 0x83,
    GetAttribLabel             = 0x84,
    DefaultMappings            = 0x85,
    FactoryReset               = 0x86,
    WriteRegister              = 0x87,
    ClearRegister              = 0x88,
    ReadRegister               = 0x89,
    GetRegisterLabel           = 0x8a,
    GetRegisterMax             = 0x8b,
    GetRegisterDefault         = 0x8c,
    SetMode                    = 0x8d,
    DefaultMouse               = 0x8e,
    ForceFeedback              = 0x8f,
    RequestCommStatus          = 0xb4,
    GetSerial                  = 0xae,
    HapticPulse                = 0xea,
}

fn main() {
    // for (p, d) in evdev::enumerate() {
    //     dbg!(p, d.name(), d.physical_path(), d.unique_name(), d.input_id(), d.properties(),
    //     d.supported_events(),
    //     d.supported_keys(),
    //     d.supported_relative_axes(),
    //     d.supported_absolute_axes(),
    //     d.supported_switches(),
    //     d.supported_ff(),
    //     );
    // }

    let api = hidapi::HidApi::new().unwrap();
    let mut found = vec![];
    let mut tries = 0;
    loop {
        eprintln!("Searching for steamdeckcontrollers...");
        for dev in api.device_list() {
            if dev.vendor_id() == 0x28de && dev.product_id() == 0x1205 && dev.interface_number() == 2 {
                found.push(dev);
            }
        }

        if found.len() > 1 {
            panic!("Too many controllers! Which one...");
        }

        if found.len() == 1 {
            break;
        }

        if tries > 10 {
            panic!("Giving up, no steamdeckcontrollers found");
        }

        //no controller found
        eprintln!("No steamdeckcontrollers found! Retrying...");
        std::thread::sleep(Duration::from_millis(500));
        tries += 1;
    }

    let dev = found.into_iter().next().unwrap().open_device(&api).expect("Could not open /dev/hidraw* device");
    dbg!(&dev);

    disable_lizard_trackpad(&dev).unwrap();

    let mut keyset = AttributeSet::new();
    for i in 1..254 {
        keyset.insert(Key(i));
    }

    let mut fake_kb = VirtualDeviceBuilder::new().expect("Could not open /dev/uinput")
    .name("padtype virtual keyboard")
    .with_keys(&*keyset).unwrap()
    .build().unwrap();

    let mut write_buf = [0u8; 65];
    write_buf[0] = Report::ClearMappings as u8;
    use std::time::{Instant, Duration};
    dev.write(write_buf.as_slice()).unwrap();
    let mut last_clear_mappings = Instant::now();

    // let mut last_a_state = false;
    let mut state:Transition<MyState> = Default::default();

    let mut input_events = vec![];

    let mut buf = [0u8; 128];
    loop {
        let len = dev.read(&mut buf).unwrap();
        let inp = parse_input_report(&buf[0..len]);
        state.push_new(inp.state());

        if state.changed() {
            if let Some(pressed) = state.change(|s| s.r2) {
                input_events.push(InputEvent::new(
                    EventType::KEY,
                    evdev::Key::KEY_LEFTSHIFT.0,
                    pressed as i32,
                ));
            }
            if let Some(pressed) = state.change(|s| s.l2) {
                input_events.push(InputEvent::new(
                    EventType::KEY,
                    evdev::Key::KEY_LEFTCTRL.0,
                    pressed as i32,
                ));
            }
            if let Some(pressed) = state.change(|s| s.l1) {
                input_events.push(InputEvent::new(
                    EventType::KEY,
                    evdev::Key::KEY_LEFTALT.0,
                    pressed as i32,
                ));
            }
        }

        for left in [false, true] {
            for octant in [
                OctantSection::Center,
                OctantSection::Octant(0),
                OctantSection::Octant(1),
                OctantSection::Octant(2),
                OctantSection::Octant(3),
                OctantSection::Octant(4),
                OctantSection::Octant(5),
                OctantSection::Octant(6),
                OctantSection::Octant(7),
            ] {
                let m = if left { map::LEFT_STICK_KEYS } else { map::RIGHT_STICK_KEYS };
                let s = if left { state.map(|s| s.get_primary_half()) } else { state.map(|s| s.get_secondary_half()) };
                let actionset = match octant {
                    OctantSection::Center => &m[0..4],
                    OctantSection::Octant(i) => &m[((i as usize)+1)*4..],
                };

                for i in 0..4 {
                    if let Some(pressed) = s.change(|s| s.stick.octant() == Some(octant) && s.buttons[i]) {
                        input_events.push(InputEvent::new(
                            EventType::KEY,
                            actionset[i].0,
                            pressed as i32,
                        ));
                    }
                }
            }
        }

        if input_events.len() > 0 {
            fake_kb.emit(&input_events).unwrap();
        }

        input_events.clear();
        // if inp.get_a() != last_a_state {
        //     let is_pressed = inp.get_a();
        //     if is_pressed {
        //         eprintln!("press a");
        //     } else {
        //         eprintln!("unpress a");
        //     }
        //     fake_kb.emit(&[InputEvent::new(EventType::KEY, evdev::Key::KEY_B.0, is_pressed as i32)]).unwrap();
        //     last_a_state = is_pressed;
        // }
        // dbg!(inp.l_stick_x, inp.l_stick_y);
        // dbg!(inp.r_pad_x, inp.r_pad_y);
        if last_clear_mappings.elapsed() > Duration::from_secs(1) {
            dev.write(write_buf.as_slice()).unwrap();
            last_clear_mappings = Instant::now();
        }
    }
    
}

use arrayref::array_ref;
use evdev::{uinput::VirtualDeviceBuilder, AttributeSet, InputEvent, EventType, Key};
use hidapi::{HidDevice, HidError};
use packed_bools::PackedBooleans;

#[derive(Debug, Copy, Clone, PackedBooleans, PartialEq, Eq, Hash)]
pub struct InputReport {
    pub unk_header: [u8; 3],
    pub size: u8,
    pub frame: u32,
    #[pack_bools(r2, l2, r1, l1, y, b, x, a)]
    pub b08: u8,
    #[pack_bools(n, w, e, s, options, steam, menu, l5)]
    pub b09: u8,
    #[pack_bools(r5, l_pad_press, r_pad_press, l_pad_touch, r_pad_touch, _unk3, l3, _unk4)]
    pub b10: u8,
    #[pack_bools(_unk5, _unk6, r3, _unk7, _unk8, _unk9, _unk10, _unk11)]
    pub b11: u8,
    pub b12: u8,
    #[pack_bools(_unk20, l4, r4, _unk21, _unk22, _unk23, l_stick_touch, r_stick_touch)]
    pub b13: u8,
    #[pack_bools(_unk24, _unk25, quick_access, _unk26, _unk27, _unk28, _unk29, _unk30)]
    pub b14: u8,
    pub b15: u8,
    pub l_pad_x: i16,
    pub l_pad_y: i16,
    pub r_pad_x: i16,
    pub r_pad_y: i16,

    pub accel_x: i16,
    pub accel_y: i16,
    pub accel_z: i16,

    pub pitch: i16,
    pub yaw:   i16,
    pub roll:  i16,

    pub _unk_gyro: [u8; 8],

    pub l_trig: u16,
    pub r_trig: u16,

    pub l_stick_x: i16,
    pub l_stick_y: i16,
    pub r_stick_x: i16,
    pub r_stick_y: i16,

    pub l_pad_force: u16,
    pub r_pad_force: u16,

    pub l_stick_force: u16,
    pub r_stick_force: u16,
}

impl InputReport {
    pub fn quads(self) -> LR<Buttons> {
        LR{
            l: Buttons{
                n: self.get_n(),
                e: self.get_e(),
                s: self.get_s(),
                w: self.get_w(),
            },
            r: Buttons{
                n: self.get_y(),
                e: self.get_b(),
                s: self.get_a(),
                w: self.get_x(),
            }
        }
    }

    pub fn sticks(self) -> LR<XY> {
        LR{
            l: XY{
                x: self.l_stick_x,
                y: self.l_stick_y,
            },
            r: XY{
                x: self.r_stick_x,
                y: self.r_stick_y,
            },
        }
    }

    pub fn pads(self) -> LR<XY> {
        LR{
            l: XY{
                x: self.l_pad_x,
                y: self.l_pad_y,
            },
            r: XY{
                x: self.r_pad_x,
                y: self.r_pad_y,
            },
        }
    }

    pub fn state(self) -> MyState {
        MyState {
            sticks: self.sticks(),
            buttons: self.quads(),
            l1: self.get_l1(),
            l2: self.get_l2(),
            r1: self.get_r1(),
            r2: self.get_r2(),
        }
    }
}

fn parse_input_report(r: &[u8]) -> InputReport {
    if r.len() != 64 {
        panic!("bad size");
    }

    if r[0..3] != [0x01, 0x00, 0x09] {
        panic!("unrecognized report type");
    }

    macro_rules! bytes_num {
        ($n:ident, $a:ident, $from:literal .. $to:literal) => {
            <$n>::from_le_bytes(*array_ref![$a, $from, $to-$from])
        }
    }

    InputReport { 
        unk_header: [r[0], r[1], r[2]], 
        size: r[3],
        frame: bytes_num!(u32, r, 4..8),
        b08: r[8],
        b09: r[9],
        b10: r[10],
        b11: r[11],
        b12: r[12],
        b13: r[13],
        b14: r[14],
        b15: r[15],
        l_pad_x: bytes_num!(i16, r, 16..18),
        l_pad_y: bytes_num!(i16, r, 18..20),
        r_pad_x: bytes_num!(i16, r, 20..22),
        r_pad_y: bytes_num!(i16, r, 22..24),
        accel_x: bytes_num!(i16, r, 24..26),
        accel_y: bytes_num!(i16, r, 26..28),
        accel_z: bytes_num!(i16, r, 28..30),
        pitch:   bytes_num!(i16, r, 30..32),
        yaw:     bytes_num!(i16, r, 32..34),
        roll:    bytes_num!(i16, r, 34..36),
        _unk_gyro: *array_ref![r, 36, 8],
        l_trig: bytes_num!(u16, r, 44..46),
        r_trig: bytes_num!(u16, r, 46..48),
        l_stick_x: bytes_num!(i16, r, 48..50),
        l_stick_y: bytes_num!(i16, r, 50..52),
        r_stick_x: bytes_num!(i16, r, 52..54),
        r_stick_y: bytes_num!(i16, r, 54..56),
        l_pad_force: bytes_num!(u16, r, 56..58),
        r_pad_force: bytes_num!(u16, r, 58..60),
        l_stick_force: bytes_num!(u16, r, 60..62),
        r_stick_force: bytes_num!(u16, r, 62..64),
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Register {
    LpadMode = 0x07,
    RpadMode = 0x08,
    RpadMargin = 0x18,
    GyroMode = 0x30,
}

fn write_register(dev: &HidDevice, register: Register, value: u16) -> Result<usize, HidError> {
    let mut write_buf = [0u8; 64];
    write_buf[0] = Report::WriteRegister as u8;
    write_buf[1] = 3; // length
    write_buf[2] = register as u8;
    let val_bytes = value.to_le_bytes();
    write_buf[3] = val_bytes[0];
    write_buf[4] = val_bytes[1];
    dev.write(write_buf.as_slice())
}

fn disable_lizard_trackpad(dev: &HidDevice) -> Result<(), HidError> {
    write_register(dev, Register::RpadMode, 0x07)?;
    write_register(dev, Register::RpadMargin, 0x00)?;
    Ok(())
}