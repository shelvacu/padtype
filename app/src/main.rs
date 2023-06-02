#![warn(clippy::all, rust_2018_idioms)]

use std::io;
use std::time::Duration;

use itermore::Itermore;
// use rusty_xinput::XInputState;
// use winput_stuffer::KeyboardLayout;
use linput_stuffer::{x11rb::{self, connection::Connection, protocol::xproto::ConnectionExt}, KeyStuffingContext, fake_input};

use x11rb::protocol::xproto::{
    ModMask,
    Keycode,
    Keysym,
};

use gilrs::{Gamepad, Gilrs, ev::state::{AxisData, ButtonData}};
// use gilrs::ev::state::GamepadState;

use linput_stuffer::sym_defs::*;

#[derive(Debug, Default, PartialEq, Eq, Clone, Hash)]
pub enum Action {
    #[default]
    None,
    /// Send this string
    Unicode(String),
    /// Press and release the key of the given name
    Key(Keysym),
    // /// Press all of the given keys in order, release in reverse order
    // Combo(Vec<String>),
}

#[derive(Default, Debug, PartialEq, Eq, Hash)]
pub struct ActionSet {
    n: Action,
    e: Action,
    s: Action,
    w: Action,
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct Polar {
    pub vel: f64, //0 .. 1
    pub dir: f64, //0 .. 1 angle, 0 is top/up
}

fn xy_to_vel_cir(x: f64, y: f64) -> Polar {
    if x == 0.0 && y == 0.0 { return Polar {vel: 0.0, dir: 0.0} }
    let vel = f64::sqrt((x*x) + (y*y));
    let vel = vel.clamp(0.0,1.0);
    let dir_rad = f64::atan2(x, y);
    let dir_cir = dir_rad / (std::f64::consts::PI * 2.0); // range is -0.5 .. 0.5 and 0 is at the top
    let dir_cir = if dir_cir < 0.0 { dir_cir + 1.0 } else { dir_cir }; //range is 0 .. 1 and 0 is at the top
    let dir_cir = dir_cir.clamp(0.0,1.0);

    Polar { vel, dir: dir_cir }
}

#[derive(Debug,Copy,Clone,PartialEq,Eq)]
enum OctantSection {
    Octant(u8),
    Center,
}

const SPACE_BETWEEN_OCTANTS_DEG:f64 = 5.0;
const PADD:f64 = (SPACE_BETWEEN_OCTANTS_DEG / 360.0) / 2.0;

fn within_padd(n: f64, from: f64, to: f64) -> bool {
    (from + PADD) <= n && n <= (to - PADD)
}

fn polar_to_octant(p: Polar) -> Option<OctantSection> {
    let p = Polar { vel: p.vel, dir: (p.dir + (1.0/16.0)) % 1.0};
    if p.vel < 0.6 {
        Some(OctantSection::Center)
    } else if within_padd(p.dir, 0.0, 1.0/8.0) {
        Some(OctantSection::Octant(0))
    } else if within_padd(p.dir, 1.0/8.0, 2.0/8.0) {
        Some(OctantSection::Octant(1))
    } else if within_padd(p.dir, 2.0/8.0, 3.0/8.0) {
        Some(OctantSection::Octant(2))
    } else if within_padd(p.dir, 3.0/8.0, 4.0/8.0) {
        Some(OctantSection::Octant(3))
    } else if within_padd(p.dir, 4.0/8.0, 5.0/8.0) {
        Some(OctantSection::Octant(4))
    } else if within_padd(p.dir, 5.0/8.0, 6.0/8.0) {
        Some(OctantSection::Octant(5))
    } else if within_padd(p.dir, 6.0/8.0, 7.0/8.0) {
        Some(OctantSection::Octant(6))
    } else if within_padd(p.dir, 7.0/8.0, 8.0/8.0) {
        Some(OctantSection::Octant(7))
    } else {
        None
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Buzz { Big, Small }

fn str_to_actionsets(s: &'static str) -> impl Iterator<Item=ActionSet> {
    s.chars()
    .array_chunks::<4>()
    .map(|[a,b,c,d]| ActionSet {
        n: Action::Unicode(a.into()),
        e: Action::Unicode(b.into()),
        s: Action::Unicode(c.into()),
        w: Action::Unicode(d.into()),
    })
}

lazy_static::lazy_static!{
    // static ref CENTERED_ACTIONSET:ActionSet = ActionSet {
    //     n: Action::Key("tab".into()),
    //     e: Action::Key("space".into()),
    //     s: Action::Key("return".into()),
    //     w: Action::Key("backspace".into()),
    // };
    // static ref CENTERED_SECOND_ACTIONSET:ActionSet = ActionSet {
    //     n: Action::Combo(vec!["control_l".into(), "x".into()]),
    //     e: Action::Combo(vec!["control_l".into(), "c".into()]),
    //     s: Action::Combo(vec!["control_l".into(), "v".into()]),
    //     w: Action::Key("delete".into()),
    // };
    static ref SHIFT:Action = Action::Key(XK_Shift_L);
    static ref CTRL:Action = Action::Key(XK_Control_L);
    static ref ALT:Action = Action::Key(XK_Alt_L);
    // static ref SUPER:Action = Action::Key("super_l".into());
    static ref DPAD_ACTIONSET:[ActionSet; 2] = [
        ActionSet {
            n: Action::Key(XK_Up),
            s: Action::Key(XK_Down),
            w: Action::Key(XK_Left),
            e: Action::Key(XK_Right),
        },
        ActionSet {
            n: Action::Key(XK_Page_Up),
            s: Action::Key(XK_Page_Down),
            w: Action::Key(XK_Home),
            e: Action::Key(XK_End),
        },
    ];
    static ref ACTIONSETS:[Vec<ActionSet>; 2] = [
        str_to_actionsets("abcdefghijklmnopqrstuvwxyz;/,.\\'").chain(
            std::iter::once(ActionSet {
                n: Action::Key(XK_Tab),
                e: Action::Key(' ' as u32),
                s: Action::Key(XK_Return),
                w: Action::Key(XK_BackSpace),
            })
        ).collect(),
        str_to_actionsets(concat!(
            "1234", // N
            "5678", // NE
            "90-=", // E
            "[]`\u{2122}" // SE
        )).chain(
            vec![
                ActionSet { // S
                    n: Action::Key(XK_Escape),
                    e: Action::Key(XK_Print),
                    s: Action::Key(XK_Insert),
                    w: Action::Key(XK_Delete),
                },
                ActionSet { // SW
                    n: Action::Key(XK_F1),
                    e: Action::Key(XK_F2),
                    s: Action::Key(XK_F3),
                    w: Action::Key(XK_F4),
                },
                ActionSet { // W
                    n: Action::Key(XK_F5),
                    e: Action::Key(XK_F6),
                    s: Action::Key(XK_F7),
                    w: Action::Key(XK_F8),
                },
                ActionSet { // NW
                    n: Action::Key(XK_F9),
                    e: Action::Key(XK_F10),
                    s: Action::Key(XK_F11),
                    w: Action::Key(XK_F12),
                },
                ActionSet { // Center
                    n: Action::Key(XK_Up),
                    e: Action::Key(XK_Right),
                    s: Action::Key(XK_Down),
                    w: Action::Key(XK_Left),
                },
            ].into_iter()
        ).collect()
    ];
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct MyState {
    pub left_x: f32,
    pub left_y: f32,
    pub right_x: f32,
    pub right_y: f32,
    pub north: bool,
    pub east: bool,
    pub south: bool,
    pub west: bool,
    pub left_trigger: bool,
    pub left_trigger_2: bool,
    pub right_trigger: bool,
    pub right_trigger_2: bool,
    pub select: bool,
    pub start: bool,
    pub left_thumb: bool,
    pub right_thumb: bool,
    pub dpad_up: bool,
    pub dpad_right: bool,
    pub dpad_down: bool,
    pub dpad_left: bool,
}

impl MyState {
    fn invert(self) -> Self {
        Self {
            right_x: self.left_x,
            right_y: self.left_y,
            left_x: self.right_x,
            left_y: self.right_y,
            dpad_up: self.north,
            dpad_right: self.east,
            dpad_down: self.south,
            dpad_left: self.west,
            north: self.dpad_up,
            east: self.dpad_right,
            south: self.dpad_down,
            west: self.dpad_left,
            ..self
        }
    }
    fn from_gamepad(g: &Gamepad) -> Self {
        Self {
            left_x: g.axis_data(gilrs::Axis::LeftStickX).map(AxisData::value).unwrap_or(0.0),
            left_y: g.axis_data(gilrs::Axis::LeftStickY).map(AxisData::value).unwrap_or(0.0),
            right_x: g.axis_data(gilrs::Axis::RightStickX).map(AxisData::value).unwrap_or(0.0),
            right_y: g.axis_data(gilrs::Axis::RightStickY).map(AxisData::value).unwrap_or(0.0),
            north: g.button_data(gilrs::Button::North).map(ButtonData::is_pressed).unwrap_or(false),
            east: g.button_data(gilrs::Button::East).map(ButtonData::is_pressed).unwrap_or(false),
            south: g.button_data(gilrs::Button::South).map(ButtonData::is_pressed).unwrap_or(false),
            west: g.button_data(gilrs::Button::West).map(ButtonData::is_pressed).unwrap_or(false),
            left_trigger: g.button_data(gilrs::Button::LeftTrigger).map(ButtonData::is_pressed).unwrap_or(false),
            left_trigger_2: g.button_data(gilrs::Button::LeftTrigger2).map(ButtonData::is_pressed).unwrap_or(false),
            right_trigger: g.button_data(gilrs::Button::RightTrigger).map(ButtonData::is_pressed).unwrap_or(false),
            right_trigger_2: g.button_data(gilrs::Button::RightTrigger2).map(ButtonData::is_pressed).unwrap_or(false),
            select: g.button_data(gilrs::Button::Select).map(ButtonData::is_pressed).unwrap_or(false),
            start: g.button_data(gilrs::Button::Start).map(ButtonData::is_pressed).unwrap_or(false),
            left_thumb: g.button_data(gilrs::Button::LeftThumb).map(ButtonData::is_pressed).unwrap_or(false),
            right_thumb: g.button_data(gilrs::Button::RightThumb).map(ButtonData::is_pressed).unwrap_or(false),
            dpad_up: g.button_data(gilrs::Button::DPadUp).map(ButtonData::is_pressed).unwrap_or(false),
            dpad_right: g.button_data(gilrs::Button::DPadRight).map(ButtonData::is_pressed).unwrap_or(false),
            dpad_down: g.button_data(gilrs::Button::DPadDown).map(ButtonData::is_pressed).unwrap_or(false),
            dpad_left: g.button_data(gilrs::Button::DPadLeft).map(ButtonData::is_pressed).unwrap_or(false),
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct TransitionState<'a> {
    pub prev: &'a MyState,
    pub curr: &'a MyState,
    pub left_prev_maybe_octant: Option<OctantSection>,
    pub left_maybe_octant: Option<OctantSection>,
    pub right_prev_maybe_octant: Option<OctantSection>,
    pub right_maybe_octant: Option<OctantSection>,
}

impl<'a> TransitionState<'a> {
    // pub fn pressed<F: FnMut(&XInputState) -> bool>(self, f: F, secondary: bool) -> bool {
    //     self.change(f, secondary).unwrap_or(false)
    // }

    // pub fn released<F: FnMut(&XInputState) -> bool>(self, f: F, secondary: bool) -> bool {
    //     !self.change(f, secondary).unwrap_or(true)
    // }

    pub fn change<F: FnMut(&MyState) -> bool>(self, mut f: F, secondary: bool) -> Option<bool> {
        let prev = if secondary {
            self.prev.invert()
        } else { *self.prev };
        let curr = if secondary {
            self.curr.invert()
        } else { *self.curr };
        let before = f(&prev);
        let now = f(&curr);
        match (before, now) {
            (false, true)  => Some(true),
            (true, false)  => Some(false),
            (true, true)   => None,
            (false, false) => None,
        }
    }

    pub fn change_nos<F: FnMut(&MyState) -> bool>(self, mut f: F) -> Option<bool> {
        let before = f(self.prev);
        let now = f(self.curr);
        match (before, now) {
            (false, true)  => Some(true),
            (true, false)  => Some(false),
            (true, true)   => None,
            (false, false) => None,
        }
    }

    pub fn octant_change<F: FnMut(&MyState) -> bool>(self, mut f: F, octant: OctantSection, secondary: bool) -> Option<bool> {
        let prev = if secondary {
            self.prev.invert()
        } else { *self.prev };
        let curr = if secondary {
            self.curr.invert()
        } else { *self.curr };
        let before = f(&prev) && Some(octant) == if !secondary { self.left_prev_maybe_octant } else { self.right_prev_maybe_octant };
        let now = f(&curr) && Some(octant) == if !secondary { self.left_maybe_octant } else { self.right_maybe_octant };

        match (before, now) {
            (false, true)  => Some(true),
            (true, false)  => Some(false),
            (true, true)   => None,
            (false, false) => None,
        }
    }
}

#[derive(Debug)]
struct SafetyDepressShiftState;

impl Drop for SafetyDepressShiftState {
    fn drop(&mut self) {
        use linput_stuffer::sym_defs::*;
        let (x_con, _) = x11rb::connect(None).unwrap();
        let mut stuffer = KeyStuffingContext::new(&x_con);
        for keysym in [XK_Alt_L, XK_Alt_R, XK_Shift_L, XK_Shift_R, XK_Super_L, XK_Super_R, XK_Menu, XK_Control_L, XK_Control_R, ' ' as u32, XK_Tab] {
            stuffer.send_keysym(keysym, false);
        }
    }
}

fn main() -> std::process::ExitCode {
    // todo: deal with disconnected device

    let _safety_depress = SafetyDepressShiftState;
    let (x_con, _) = x11rb::connect(None).unwrap();

    let (act_send, act_recv) = crossbeam::channel::bounded(4);
    std::thread::spawn(move || {
        let (x_con, _) = x11rb::connect(None).unwrap();
        let mut stuffer = KeyStuffingContext::new(&x_con);
        while let Ok(acts) = act_recv.recv() {
            for (act, pressed) in acts {
                let act:&Action = act;
                match act {
                    Action::None => (),
                    Action::Key(keysym) => stuffer.send_keysym(*keysym, pressed),
                    Action::Unicode(s) => if pressed { stuffer.send_string(s.as_str()) }
                }
            }
        }
    });

    let mut action_queue:Vec<(&Action, bool)> = vec![];

    let mut gilrs = Gilrs::new().unwrap();

    let mut the_pad_id = None;
    // Iterate over all connected gamepads
    for (id, gamepad) in gilrs.gamepads() {
        if the_pad_id.is_none() { the_pad_id = Some(id) };
        println!("{} is {:?}", gamepad.name(), gamepad.power_info());
        dbg!(gamepad.state());
    }

    let the_pad_id = the_pad_id.unwrap();

    let mut prev_state = MyState::from_gamepad(&gilrs.gamepad(the_pad_id));
    let mut left_prev_maybe_octant = None;
    let mut right_prev_maybe_octant = None;

    println!("Running...");
    loop {
        let start = std::time::Instant::now();
        let _ = gilrs.next_event();
        let state = MyState::from_gamepad(&gilrs.gamepad(the_pad_id));
        if prev_state != state {
            dbg!(state);
        }

        if state.start {
            return std::process::ExitCode::SUCCESS;
        }

        if state != prev_state {
            let left_maybe_octant = {let x = state.left_x as f64;
            let y = state.left_y as f64;

            let p = xy_to_vel_cir(x, y);
            polar_to_octant(p)};
            let right_maybe_octant = {let x = state.right_x as f64;
            let y = state.right_y as f64;

            let p = xy_to_vel_cir(x, y);
            polar_to_octant(p)};

            let mut did_action = false;
            let xmsn = TransitionState{prev: &prev_state, curr: &state, left_prev_maybe_octant, left_maybe_octant, right_prev_maybe_octant, right_maybe_octant};

            if let Some(pressed) = xmsn.change_nos(|s| s.left_trigger) {
                action_queue.push((&*SHIFT, pressed));
            }
            if let Some(pressed) = xmsn.change_nos(|s| s.left_trigger_2) {
                action_queue.push((&*CTRL, pressed));
            }
            if let Some(pressed) = xmsn.change_nos(|s| s.right_trigger_2) {
                action_queue.push((&*ALT, pressed));
            }

            // for secondary in [false, true] {
            //     if let Some(pressed) = xmsn.change(|s| s.dpad_up, secondary) {
            //         action_queue.push((&DPAD_ACTIONSET[secondary as usize].n, pressed));
            //         did_action = did_action || pressed;
            //     }
            //     if let Some(pressed) = xmsn.change(|s| s.dpad_right, secondary) {
            //         action_queue.push((&DPAD_ACTIONSET[secondary as usize].e, pressed));
            //         did_action = did_action || pressed;
            //     }
            //     if let Some(pressed) = xmsn.change(|s| s.dpad_down, secondary) {
            //         action_queue.push((&DPAD_ACTIONSET[secondary as usize].s, pressed));
            //         did_action = did_action || pressed;
            //     }
            //     if let Some(pressed) = xmsn.change(|s| s.dpad_left, secondary) {
            //         action_queue.push((&DPAD_ACTIONSET[secondary as usize].w, pressed));
            //         did_action = did_action || pressed;
            //     }
            // }

            // https://stackoverflow.com/a/16991789/1267729
            //  1/2/3 being left/middle/right buttons, 4/5 and 6/7 should do vertical and horizontal wheel scrolls.
            // if let Some(pressed) = xmsn.change(|s| s.west, false) {
            //     let ev = linput_stuffer::ButtonEvent{
            //         press: pressed,
            //         button: 1,
            //     };
            //     fake_input(&x_con, ev);
            // }

            // if let Some(pressed) = xmsn.change(|s| s.east, false) {
            //     let ev = linput_stuffer::ButtonEvent{
            //         press: pressed,
            //         button: 3,
            //     };
            //     fake_input(&x_con, ev);
            // }

            for secondary in [false, true] {
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
                    let actionset = match octant {
                        OctantSection::Center => &ACTIONSETS[secondary as usize][8],
                        OctantSection::Octant(i) => &ACTIONSETS[secondary as usize][i as usize],
                    };
                    
                    if let Some(pressed) = xmsn.octant_change(|s| s.north, octant, secondary) {
                        
                        action_queue.push((&actionset.n, pressed));
                        did_action = did_action || pressed;
                    }
                    if let Some(pressed) = xmsn.octant_change(|s| s.east, octant, secondary) {
                        action_queue.push((&actionset.e, pressed));
                        did_action = did_action || pressed;
                    }
                    if let Some(pressed) = xmsn.octant_change(|s| s.south, octant, secondary) {
                        action_queue.push((&actionset.s, pressed));
                        did_action = did_action || pressed;
                    }
                    if let Some(pressed) = xmsn.octant_change(|s| s.west, octant, secondary) {
                        action_queue.push((&actionset.w, pressed));
                        did_action = did_action || pressed;
                    }
                }
            }

            // if did_action {
            //     use gilrs::ff::*;
            //     let duration = Ticks::from_ms(150);
            //     let effect = EffectBuilder::new()
            //         .add_effect(BaseEffect {
            //             kind: BaseEffectType::Strong { magnitude: 60_000 },
            //             scheduling: Replay { play_for: duration, with_delay: duration * 3, ..Default::default() },
            //             envelope: Default::default(),
            //         })
            //         .add_effect(BaseEffect {
            //             kind: BaseEffectType::Weak { magnitude: 60_000 },
            //             scheduling: Replay { after: duration * 2, play_for: duration, with_delay: duration * 3 },
            //             ..Default::default()
            //         })
            //         .gamepads(&[the_pad_id])
            //         .finish(&mut gilrs).unwrap();
            //     effect.play().unwrap();
            // }

            left_prev_maybe_octant = left_maybe_octant;
            right_prev_maybe_octant = right_maybe_octant;
        }
        if !action_queue.is_empty(){
            action_queue.sort_by_key(|(_, pressed)| *pressed as u8);
            let mut swap_queue = vec![];
            std::mem::swap(&mut action_queue, &mut swap_queue);
            act_send.send(swap_queue).unwrap();
        }
        prev_state = state;

        let run_duration = start.elapsed();
        if run_duration < Duration::from_millis(1) {
            std::thread::sleep(Duration::from_millis(1) - run_duration);
        }
    }
}
