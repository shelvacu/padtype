#![warn(clippy::all, rust_2018_idioms)]
//#![windows_subsystem = "windows"]

use itermore::Itermore;

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
    /// Do all the actions given, in order
    Macro(Vec<Action>),
}

// impl std::default::Default for Action {
//     fn default() -> Self {
//         Action::None
//     }
// }

#[derive(Default, Debug, PartialEq, Eq, Hash)]
pub struct ActionSet {
    lbutton: Action,
    rbutton: Action,
    n: Action,
    e: Action,
    s: Action,
    w: Action,
}

// use windows::Win32::UI::Input::XboxController as xc;
// use windows::Win32::Foundation as foundation;

// fn get_state(user_idx: u8) -> Result<xc::XINPUT_STATE, windows::core::Error> {
//     let mut state:xc::XINPUT_STATE = unsafe { std::mem::zeroed() };
//     let res = unsafe { xc::XInputGetState(user_idx as u32, &mut state as *mut xc::XINPUT_STATE) };
//     dbg!(res);
//     let res = foundation::WIN32_ERROR(res);
//     res.ok().map(|()| state)
// }

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

fn polar_to_octant(p: Polar) -> Option<u8> {
    let p = Polar { vel: p.vel, dir: (p.dir + (1.0/16.0)) % 1.0};
    if p.vel < 0.7 {
        None
    } else if p.dir < 0.125 {
        Some(0)
    } else if p.dir < 0.25 {
        Some(1)
    } else if p.dir < 0.375 {
        Some(2)
    } else if p.dir < 0.5 {
        Some(3)
    } else if p.dir < 0.625 {
        Some(4)
    } else if p.dir < 0.75 {
        Some(5)
    } else if p.dir < 0.875 {
        Some(6)
    } else {
        Some(7)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Buzz { Big, Small }

fn execute_action(a: &Action) {
    dbg!(a);
    match a {
        Action::None => (),
        Action::Unicode(c) => winput_stuffer::send::send_text(c).unwrap(),
        Action::Key(keyname) => {
            winput_stuffer::send::send_key(keyname, true).unwrap();
            winput_stuffer::send::send_key(keyname, false).unwrap();
        },
        _ => unimplemented!(),
    };
}

fn str_to_actionsets(s: &'static str) -> Vec<ActionSet> {
    s.chars()
    .array_chunks::<4>()
    .map(|[a,b,c,d]| ActionSet {
        w: Action::Unicode(a.into()),
        n: Action::Unicode(b.into()),
        e: Action::Unicode(c.into()),
        s: Action::Unicode(d.into()),
        .. ActionSet::default()
    })
    .collect()
}

lazy_static::lazy_static!{
    static ref ACTIONSETS:Vec<ActionSet> = str_to_actionsets("abcdefghijklmnopqrstuvwxyz,.;/@-");
    static ref SHIFT_ACTIONSETS:Vec<ActionSet> = str_to_actionsets("ABCDEFGHIJKLMNOPQRSTUVWXYZ<>:?!_");
    static ref CENTERED_ACTIONSET:ActionSet = ActionSet {
        s: Action::Key("return".into()),
        w: Action::Key("space".into()),
        n: Action::None, // Undo/Ctrl+Z
        e: Action::Key("backspace".into()),
        ..ActionSet::default()
    };
}

fn main() {
    // let alphabet:&str = "abcdefghijklmnopqrstuvwxyz,.;/@-";
    // let actionsets:Vec<ActionSet> = alphabet
    //     .chars()
    //     .array_chunks::<4>()
    //     .map(|[a,b,c,d]| ActionSet {
    //         w: Action::Unicode(a.into()),
    //         n: Action::Unicode(b.into()),
    //         e: Action::Unicode(c.into()),
    //         s: Action::Unicode(d.into()),
    //         .. ActionSet::default()
    //     })
    //     .collect();
    // let shift_alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ<>:?!_";
    // let shift_actionsets:Vec<ActionSet> = shift_alphabet
    //     .chars()
    //     .array_chunks::<4>()
    //     .map(|[a,b,c,d]| ActionSet {
    //         w: Action::Unicode(a.into()),
    //         n: Action::Unicode(b.into()),
    //         e: Action::Unicode(c.into()),
    //         s: Action::Unicode(d.into()),
    //         .. ActionSet::default()
    //     })
    //     .collect();   
    // let centered_actionset = ActionSet {
    //     s: Action::Key("return".into()),
    //     w: Action::Key("space".into()),
    //     n: Action::None, // Undo/Ctrl+Z
    //     e: Action::Key("backspace".into()),
    //     ..ActionSet::default()
    // };
    // unsafe {  windows::Win32::System::WinRT::RoInitialize(windows::Win32::System::WinRT::RO_INIT_MULTITHREADED).unwrap(); }
    // unsafe { xc::XInputEnable(true); }
    // std::thread::sleep_ms(1000);
    // dbg!(foundation::ERROR_DEVICE_NOT_CONNECTED);
    // // loop {
    //     dbg!(get_state(0));
    //     dbg!(get_state(1));
    //     dbg!(get_state(2));
    //     dbg!(get_state(3));
    // // }

    let (act_send, act_recv) = crossbeam::channel::bounded(32);
    std::thread::spawn(move || {
        while let Ok(act) = act_recv.recv() {
            execute_action(act);
        }
    });

    let do_action = move |act| act_send.send(act).unwrap();

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
    let mut prev_octant = None;
    dbg!(winput_stuffer::layout::KeyboardLayout::current().keyname_to_vk());
    println!("Running...");
    loop {
        std::thread::sleep(std::time::Duration::from_millis(1));
        let state = handle.get_state(0).unwrap();
        if state != prev_state {
            let (xi,yi) = state.left_stick_raw();
            // println!("{},{}", x, y);
            let x = (xi as f64) / (i16::MAX as f64);
            let y = (yi as f64) / (i16::MAX as f64);
            let p = xy_to_vel_cir(x, y);
            let octant = polar_to_octant(p);
            // println!("{:.5},{:.5},{:?}", p.vel, p.dir, octant);
            let mut did_action = false;

            if (prev_octant != octant && octant.is_some())
            || (!prev_state.north_button() && state.north_button())
            || (!prev_state.east_button()  && state.east_button())
            || (!prev_state.south_button() && state.south_button())
            || (!prev_state.west_button()  && state.west_button())
            {
                let set = if state.left_trigger_bool() {
                    match octant {
                        None => &CENTERED_ACTIONSET,
                        Some(i) => &SHIFT_ACTIONSETS[usize::from(i)],
                    }
                } else {
                    match octant {
                        None => &CENTERED_ACTIONSET,
                        Some(i) => &ACTIONSETS[usize::from(i)],
                    }
                };
                if state.north_button() {
                    do_action(&set.n);
                    did_action = true;
                }
                if state.east_button() {
                    do_action(&set.e);
                    did_action = true;
                }
                if state.south_button() {
                    do_action(&set.s);
                    did_action = true;
                }
                if state.west_button() {
                    do_action(&set.w);
                    did_action = true;
                }
            }

            if octant != prev_octant {
                let _ = buzz_send.try_send(
                    if did_action {
                        Buzz::Big
                    } else {
                        Buzz::Small
                    }
                );
            }
            prev_octant = octant;
            // if !prev_state.west_button() && state.west_button() {
            //     std::process::exit(0);
            // }
            // if y > 0.0 {
            //     if !prev_state.south_button() && state.south_button() {
            //         let thread_handle = handle.clone();
            //         std::thread::spawn(move || {
            //             let handle = thread_handle;
            //             handle.set_state(0, u16::MAX, 0);
            //             let dur = std::time::Duration::from_secs_f64(y * y);
            //             dbg!(dur);
            //             std::thread::sleep(dur);
            //             handle.set_state(0, 0, 0);
            //         });
            //     }
            //     if !prev_state.north_button() && state.north_button() {
            //         let thread_handle = handle.clone();
            //         std::thread::spawn(move || {
            //             let handle = thread_handle;
            //             handle.set_state(0, 0, u16::MAX);
            //             let dur = std::time::Duration::from_secs_f64(y * y);
            //             dbg!(dur);
            //             std::thread::sleep(dur);
            //             handle.set_state(0, 0, 0);
            //         });
            //     }
            //     if !prev_state.east_button() && state.east_button() {
            //         let thread_handle = handle.clone();
            //         std::thread::spawn(move || {
            //             let handle = thread_handle;
            //             handle.set_state(0, u16::MAX, u16::MAX);
            //             let dur = std::time::Duration::from_secs_f64((y * y)/10.0);
            //             dbg!(dur);
            //             std::thread::sleep(dur);
            //             handle.set_state(0, 0, 0);
            //         });
            //     }
            // }
        }
        prev_state = state;
    }
}
