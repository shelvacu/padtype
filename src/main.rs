#![warn(clippy::all, rust_2018_idioms)]

use std::io;
use std::time::Duration;

use itermore::Itermore;
use rusty_xinput::XInputState;
use winput_stuffer::KeyboardLayout;

#[derive(Debug, Default, PartialEq, Eq, Clone, Hash)]
pub enum Action {
    #[default]
    None,
    /// Send this string
    Unicode(String),
    /// Press and release the key of the given name
    Key(String),
    /// Press all of the given keys in order, release in reverse order
    Combo(Vec<String>),
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

fn action_extend_inputs(a: &Action, pressed: bool, layout: &KeyboardLayout, inputs_out: &mut Vec<winput_stuffer::input::Input>) {
    dbg!(a, pressed);
    match a {
        Action::None => (),
        Action::Unicode(text) => {
            let c = text.chars().next();
            if text.len() == 1 && layout.char_to_vk_ss().get(&c.unwrap()).map(|(_vk, ss)| *ss == 0).unwrap_or(false) {
                let vk = layout.char_to_vk_ss()[&c.unwrap()].0;
                inputs_out.push(
                    winput_stuffer::input::Input::from_keyboard(&winput_stuffer::send::key_event(vk, pressed, None).into())
                );
            } else if pressed {
                winput_stuffer::send::inputs_for_text(
                    text.as_str(),
                    layout,
                    inputs_out,
                );
            }
        }
        Action::Key(keyname) => {
            inputs_out.push(winput_stuffer::send::input_for_key(keyname, pressed, layout));
        },
        Action::Combo(keys) => {
            if pressed {
                for k in keys {
                    if k.len() == 1 {
                        winput_stuffer::send::inputs_for_text(
                            k.as_str(),
                            layout,
                            inputs_out,
                        );
                    } else {
                        inputs_out.push(winput_stuffer::send::input_for_key(k, true, layout));
                    }
                }
            } else {
                for k in keys.iter().rev() {
                    if k.len() != 1 {
                        inputs_out.push(winput_stuffer::send::input_for_key(k, false, layout));
                    }
                }
            }
        },
    };
}

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
    static ref SHIFT:Action = Action::Key("shift_l".into());
    static ref CTRL:Action = Action::Key("control_l".into());
    static ref ALT:Action = Action::Key("alt_l".into());
    // static ref SUPER:Action = Action::Key("super_l".into());
    static ref DPAD_ACTIONSET:[ActionSet; 2] = [
        ActionSet {
            n: Action::Key("up".into()),
            s: Action::Key("down".into()),
            w: Action::Key("left".into()),
            e: Action::Key("right".into()),
        },
        ActionSet {
            n: Action::Key("page_up".into()),
            s: Action::Key("page_down".into()),
            w: Action::Key("home".into()),
            e: Action::Key("end".into()),
        },
    ];
    static ref ACTIONSETS:[Vec<ActionSet>; 2] = [
        str_to_actionsets("abcdefghijklmnopqrstuvwxyz;/,.\\'").chain(
            std::iter::once(ActionSet {
                n: Action::Key("tab".into()),
                e: Action::Key("space".into()),
                s: Action::Key("return".into()),
                w: Action::Key("backspace".into()),
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
                    n: Action::Key("escape".into()),
                    e: Action::Key("print".into()),
                    s: Action::Key("insert".into()),
                    w: Action::Key("super_l".into()),
                },
                ActionSet { // SW
                    n: Action::Key("f1".into()),
                    e: Action::Key("f2".into()),
                    s: Action::Key("f3".into()),
                    w: Action::Key("f4".into()),
                },
                ActionSet { // W
                    n: Action::Key("f5".into()),
                    e: Action::Key("f6".into()),
                    s: Action::Key("f7".into()),
                    w: Action::Key("f8".into()),
                },
                ActionSet { // NW
                    n: Action::Key("f9".into()),
                    e: Action::Key("f10".into()),
                    s: Action::Key("f11".into()),
                    w: Action::Key("f12".into()),
                },
                ActionSet { // Center
                    n: Action::Combo(vec!["control_l".into(), "x".into()]),
                    e: Action::Combo(vec!["control_l".into(), "c".into()]),
                    s: Action::Combo(vec!["control_l".into(), "v".into()]),
                    w: Action::Key("delete".into()),
                },
            ].into_iter()
        ).collect()
    ];
}

#[derive(Debug, Copy, Clone)]
struct TransitionState<'a> {
    pub prev: &'a XInputState,
    pub curr: &'a XInputState,
    pub prev_maybe_octant: Option<OctantSection>,
    pub maybe_octant: Option<OctantSection>,
}

impl<'a> TransitionState<'a> {
    // pub fn pressed<F: FnMut(&XInputState) -> bool>(self, f: F, secondary: bool) -> bool {
    //     self.change(f, secondary).unwrap_or(false)
    // }

    // pub fn released<F: FnMut(&XInputState) -> bool>(self, f: F, secondary: bool) -> bool {
    //     !self.change(f, secondary).unwrap_or(true)
    // }

    pub fn change<F: FnMut(&XInputState) -> bool>(self, mut f: F, secondary: bool) -> Option<bool> {
        let before = f(self.prev) && (secondary == self.prev.right_trigger_bool());
        let now = f(self.curr) && (secondary == self.curr.right_trigger_bool());
        match (before, now) {
            (false, true)  => Some(true),
            (true, false)  => Some(false),
            (true, true)   => None,
            (false, false) => None,
        }
    }

    pub fn change_nos<F: FnMut(&XInputState) -> bool>(self, mut f: F) -> Option<bool> {
        let before = f(self.prev);
        let now = f(self.curr);
        match (before, now) {
            (false, true)  => Some(true),
            (true, false)  => Some(false),
            (true, true)   => None,
            (false, false) => None,
        }
    }

    pub fn octant_change<F: FnMut(&XInputState) -> bool>(self, mut f: F, octant: OctantSection, secondary: bool) -> Option<bool> {
        let before = f(self.prev) && Some(octant) == self.prev_maybe_octant && (secondary == self.prev.right_trigger_bool());
        let now = f(self.curr) && Some(octant) == self.maybe_octant && (secondary == self.curr.right_trigger_bool());
        match (before, now) {
            (false, true)  => Some(true),
            (true, false)  => Some(false),
            (true, true)   => None,
            (false, false) => None,
        }
    }
}

#[derive(Debug)]
struct SafetyDepressShiftState {}

impl Drop for SafetyDepressShiftState {
    fn drop(&mut self) {
        use winput_stuffer::layout::maps::*;
        let layout = KeyboardLayout::current();
        for kn in ["super_l", "shift_l", "control_l", "alt_l", "super_r", "shift_r", "control_r", "alt_r"] {
            let i = winput_stuffer::send::input_for_key(kn, false, &layout);
            let vec = vec![i];
            let _ = winput_stuffer::input::send_input(vec.as_slice());
        }
        for vk in [VK_LWIN, VK_LSHIFT, VK_LCONTROL, VK_LMENU, VK_RWIN, VK_RSHIFT, VK_RCONTROL, VK_RMENU, VK_SHIFT, VK_CONTROL, VK_MENU, VK_SPACE, VK_TAB,] {
            let ki = winput_stuffer::send::key_event(vk, false, None);
            let vec = vec![winput_stuffer::input::Input::from_keyboard(&ki.into())];
            //Ignore this since we're in a drop impl
            let _ = winput_stuffer::input::send_input(vec.as_slice());
        }
    }
}

fn main() -> std::process::ExitCode {
    // todo: deal with disconnected device

    let _safety_depress = SafetyDepressShiftState {};

    let (act_send, act_recv) = crossbeam::channel::bounded(4);
    std::thread::spawn(move || {
        let mut inputs = vec![];
        while let Ok(acts) = act_recv.recv() {
            let layout = KeyboardLayout::current();
            for (act,pressed) in acts {
                action_extend_inputs(act, pressed, &layout, &mut inputs);
            }
            winput_stuffer::input::send_input(inputs.as_slice()).map(|count| assert_eq!(inputs.len() as u32, count)).unwrap();
            inputs.clear();
        }
    });

    let mut action_queue:Vec<(&Action, bool)> = vec![];

    let handle = rusty_xinput::XInputHandle::load_default().unwrap();
    handle.enable(true);

    let thread_handle = handle.clone();
    let (buzz_send, buzz_recv) = crossbeam::channel::bounded(2);
    std::thread::spawn(move || {
        let handle = thread_handle;
        let dur = std::time::Duration::from_millis(1);
        while let Ok(buzz) = buzz_recv.recv() {
            let power = match buzz {
                Buzz::Big => 30_000,
                Buzz::Small => 15_000,
            };
            handle.set_state(0, 0, power).unwrap();
            std::thread::sleep(dur);
            handle.set_state(0, 0, 0).unwrap();
            std::thread::sleep(dur);
        }
    });

    let mut prev_state = handle.get_state(0).unwrap();
    let mut prev_maybe_octant = None;
    let mut keys:Vec<_> = winput_stuffer::layout::KeyboardLayout::current()
        .keyname_to_vk()
        .iter()
        .map(|(a, _)| a.clone())
        .collect();
    keys.sort();
    dbg!(keys);
    println!("Running...");
    loop {
        let start = std::time::Instant::now();
        let state = handle.get_state(0).unwrap();

        if state.start_button() {
            return std::process::ExitCode::SUCCESS;
        }

        let rs = state.right_stick_raw();
        let mouse_move = rusty_xinput::XInputState::normalize_raw_stick_value(rs, 0);

        if mouse_move != (0.0, 0.0) {
            let factor:f32 = 50.0;
            use winput_stuffer::input::*;
            let mouse = MouseInput {
                e: MouseInputEnum::Move{
                    coalesce: true,
                    m: MouseMovement::Relative{
                        dx:  (mouse_move.0.powf(2.0) * factor).copysign(mouse_move.0) as i32,
                        dy: -(mouse_move.1.powf(2.0) * factor).copysign(mouse_move.1) as i32,
                    },
                },
                msg: None,
                time: None,
            };
            let input = Input::from_mouse(&mouse.into());
            let inputs = [input];
            send_input(inputs.as_slice()).unwrap();
        }

        if state != prev_state {
            let (xi,yi) = state.left_stick_raw();
            // println!("{},{}", x, y);
            let x = (xi as f64) / (i16::MAX as f64);
            let y = (yi as f64) / (i16::MAX as f64);
            let p = xy_to_vel_cir(x, y);
            let maybe_octant = polar_to_octant(p);
            // println!("{:.5},{:.5},{:?}", p.vel, p.dir, octant);
            let mut did_action = false;
            let xmsn = TransitionState{prev: &prev_state, curr: &state, prev_maybe_octant, maybe_octant};

            if let Some(pressed) = xmsn.change_nos(XInputState::left_trigger_bool) {
                action_queue.push((&*SHIFT, pressed));
            }
            if let Some(pressed) = xmsn.change_nos(XInputState::left_shoulder) {
                action_queue.push((&*CTRL, pressed));
            }
            if let Some(pressed) = xmsn.change_nos(XInputState::right_shoulder) {
                action_queue.push((&*ALT, pressed));
            }

            for secondary in [false, true] {
                if let Some(pressed) = xmsn.change(XInputState::arrow_up, secondary) {
                    action_queue.push((&DPAD_ACTIONSET[secondary as usize].n, pressed));
                    did_action = did_action || pressed;
                }
                if let Some(pressed) = xmsn.change(XInputState::arrow_right, secondary) {
                    action_queue.push((&DPAD_ACTIONSET[secondary as usize].e, pressed));
                    did_action = did_action || pressed;
                }
                if let Some(pressed) = xmsn.change(XInputState::arrow_down, secondary) {
                    action_queue.push((&DPAD_ACTIONSET[secondary as usize].s, pressed));
                    did_action = did_action || pressed;
                }
                if let Some(pressed) = xmsn.change(XInputState::arrow_left, secondary) {
                    action_queue.push((&DPAD_ACTIONSET[secondary as usize].w, pressed));
                    did_action = did_action || pressed;
                }
            }

            if let Some(pressed) = xmsn.change(XInputState::left_thumb_button, false) {
                use winput_stuffer::input::*;
                let mouse = MouseInput {
                    e: MouseInputEnum::Button{
                        which: MouseButton::Left,
                        button_up: !pressed,
                    },
                    msg: None,
                    time: None,
                };
                let input = Input::from_mouse(&mouse.into());
                let inputs = [input];
                send_input(inputs.as_slice()).unwrap();
            }

            if let Some(pressed) = xmsn.change(XInputState::right_thumb_button, false) {
                use winput_stuffer::input::*;
                let mouse = MouseInput {
                    e: MouseInputEnum::Button{
                        which: MouseButton::Right,
                        button_up: !pressed,
                    },
                    msg: None,
                    time: None,
                };
                let input = Input::from_mouse(&mouse.into());
                let inputs = [input];
                send_input(inputs.as_slice()).unwrap();
            }

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
                    
                    if let Some(pressed) = xmsn.octant_change(XInputState::north_button, octant, secondary) {
                        
                        action_queue.push((&actionset.n, pressed));
                        did_action = did_action || pressed;
                    }
                    if let Some(pressed) = xmsn.octant_change(XInputState::east_button, octant, secondary) {
                        action_queue.push((&actionset.e, pressed));
                        did_action = did_action || pressed;
                    }
                    if let Some(pressed) = xmsn.octant_change(XInputState::south_button, octant, secondary) {
                        action_queue.push((&actionset.s, pressed));
                        did_action = did_action || pressed;
                    }
                    if let Some(pressed) = xmsn.octant_change(XInputState::west_button, octant, secondary) {
                        action_queue.push((&actionset.w, pressed));
                        did_action = did_action || pressed;
                    }
                }
            }

            if did_action {
                let _ = buzz_send.try_send(
                    if did_action {
                        Buzz::Big
                    } else {
                        Buzz::Small
                    }
                );
            }

            // if let Some(octant) = maybe_octant {
            //     if (prev_octant != octant && matches!(octant, OctantSection::Octant(_)))
            //     || (!prev_state.north_button() && state.north_button())
            //     || (!prev_state.east_button()  && state.east_button())
            //     || (!prev_state.south_button() && state.south_button())
            //     || (!prev_state.west_button()  && state.west_button())
            //     {
            //         let set = if state.left_trigger_bool() {
            //             match octant {
            //                 OctantSection::Center => &CENTERED_ACTIONSET,
            //                 OctantSection::Octant(i) => &SHIFT_ACTIONSETS[usize::from(i)],
            //             }
            //         } else {
            //             match octant {
            //                 OctantSection::Center => &CENTERED_ACTIONSET,
            //                 OctantSection::Octant(i) => &ACTIONSETS[usize::from(i)],
            //             }
            //         };
            //         if state.north_button() {
            //             do_action(&set.n);
            //             did_action = true;
            //         }
            //         if state.east_button() {
            //             do_action(&set.e);
            //             did_action = true;
            //         }
            //         if state.south_button() {
            //             do_action(&set.s);
            //             did_action = true;
            //         }
            //         if state.west_button() {
            //             do_action(&set.w);
            //             did_action = true;
            //         }
            //     }

            // }
            prev_maybe_octant = maybe_octant;
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
