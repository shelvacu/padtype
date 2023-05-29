pub use x11rb;

pub mod sym_defs;
mod unicode_to_keysym;

use std::collections::HashMap;
use std::ops::Index;
use std::collections::VecDeque;

use x11rb::protocol::xtest::FakeInputRequest;
use x11rb::rust_connection::Stream;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    ModMask,
    Keycode,
    Keysym,
};
use x11rb::protocol::xtest::ConnectionExt as aaaaaa;
use x11rb::protocol::xproto::ConnectionExt as aaa;

// FAKE_EVENT_TYPE
// 2     KeyPress
// 3     KeyRelease
// 4     ButtonPress
// 5     ButtonRelease
// 6     MotionNotify

pub struct KeyEvent {
    press: bool,
    keycode: u8, //???
}

impl From<KeyEvent> for FakeInputRequest {
    fn from(ev: KeyEvent) -> FakeInputRequest {
        FakeInputRequest {
            type_: if ev.press { 2 } else { 3 },
            detail: ev.keycode,
            time: 0,
            // Should all be unused:
            root: 0,
            root_x: 0,
            root_y: 0,
            deviceid: 0,
        }
    }
}

pub struct ButtonEvent {
    press: bool,
    // "this field is interpreted as the physical (or core) button, meaning it will be mapped to the corresponding logical button according to the most recent SetPointerMapping request." 
    button: u8,
}

impl From<ButtonEvent> for FakeInputRequest {
    fn from(ev: ButtonEvent) -> FakeInputRequest {
        FakeInputRequest {
            type_: if ev.press { 4 } else { 5 },
            detail: ev.button,
            time: 0,
            // Should all be unused:
            root: 0,
            root_x: 0,
            root_y: 0,
            deviceid: 0,
        }
    }
}

pub struct MotionEvent {
    x: i16,
    y: i16,
    relative: bool, // detail field
}

impl From<MotionEvent> for FakeInputRequest {
    fn from(ev: MotionEvent) -> FakeInputRequest {
        FakeInputRequest {
            type_: 6,
            detail: ev.relative as u8,
            time: 0,
            // Should all be unused:
            root: 0,
            root_x: ev.x,
            root_y: ev.y,
            deviceid: 0,
        }
    }
}

pub fn fake_input<C: Connection>(conn: &C, into_req: impl Into<FakeInputRequest>) {
    let req:FakeInputRequest = into_req.into();
    conn.xtest_fake_input(
        req.type_,
        req.detail,
        req.time,
        req.root,
        req.root_x,
        req.root_y,
        req.deviceid,
    ).unwrap().check().unwrap();
}

// Shamelessly copied most of this from https://github.com/openstenoproject/plover/blob/master/plover/oslayer/linux/keyboardcontrol_x11.py

pub struct KeyStuffingContext<'a, C: Connection> {
    conn: &'a C,
    keymap: HashMap<Keysym, Mapping>,
    // really this should be something like an LRU cache, but I don't care
    custom_mappings_queue: VecDeque<Mapping>,
    backspace: Mapping,
    modifier_map: ModifierMapping,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mapping {
    keycode: Keycode,
    modifiers: ModMask,
    keysym: Keysym,
    //custom_mapping: ???
}

pub struct ModifierMapping {
    subsize: usize,
    map: Vec<u8>,
}

impl Index<usize> for ModifierMapping {
    type Output = [u8];

    fn index(&self, idx: usize) -> &Self::Output {
        if idx > 7 {
            panic!("Too big");
        }
        &self.map[idx*self.subsize..(idx+1)*self.subsize]
    }
}

const UNUSED_KEYSYM:u32 = u32::MAX;

impl<'a, C: Connection> KeyStuffingContext<'a, C> {
    pub fn new(conn: &'a C) -> Self {
        let mut keymap = HashMap::new();
        let mut custom_mappings_queue = VecDeque::new();

        let min_keycode = conn.setup().min_keycode;
        let keycode_count = conn.setup().max_keycode - min_keycode + 1;
        let km = conn.get_keyboard_mapping(min_keycode, keycode_count).unwrap().reply().unwrap();
        for (idx, mut keysyms) in km.keysyms.chunks(km.keysyms_per_keycode.into()).enumerate() {
            let keycode = min_keycode + (idx as u8);
            while let Some((&x11rb::NO_SYMBOL, new_keysyms)) = keysyms.split_last() {
                keysyms = new_keysyms;
            }

            if keysyms.is_empty() {
                custom_mappings_queue.push_front(Mapping{
                    keycode,
                    modifiers: ModMask::default(),
                    keysym: UNUSED_KEYSYM,
                });
                // custom_mappings_queue.push_front(Mapping{
                //     keycode,
                //     ModMask::SHIFT,
                //     UNUSED_KEYSYM,
                // });
            }

            for (idx, keysym) in keysyms.iter().copied().enumerate() {
                if idx != 0 && idx != 1 && idx != 4 && idx != 5 { break; }
                let mut modifiers = ModMask::default();
                if idx % 2 == 1 {
                    // keycode needs shift
                    modifiers |= ModMask::SHIFT;
                }
                if idx == 4 || idx == 5 {
                    modifiers |= ModMask::M5;
                }
                let mapping = Mapping{
                    keycode,
                    modifiers,
                    keysym,
                };

                if keysym != x11rb::NO_SYMBOL {
                    let maybe = keymap.get(&keysym);
                    match maybe {
                        None => {
                            keymap.insert(keysym, mapping);
                        },
                        Some(v) if v.modifiers > modifiers => {
                            keymap.insert(keysym, mapping);
                        },
                        _ => (),
                    }
                }
            }
        }

        let backspace = *keymap.get(&sym_defs::XK_BackSpace).unwrap();
        //get_modifier_mapping
        // python gets "a list of eight lists, one for each modifier. The list can be indexed using X.ShiftMapIndex, X.Mod1MapIndex, and so on. The sublists list the keycodes bound to that modifier."
        // We get a Vec<u8>, I guess we gotta chop it up ourselves
        let raw_mod_map = conn.get_modifier_mapping().unwrap().reply().unwrap().keycodes;
        if raw_mod_map.len() == 0 {
            panic!("Nah");
        }
        if raw_mod_map.len() % 8 != 0 {
            panic!("GetModifierMapping returned a number of keycodes which is not a multiple of 8");
        }

        conn.flush().unwrap();
        Self{
            conn,
            keymap,
            custom_mappings_queue,
            backspace,
            modifier_map: ModifierMapping{
                subsize: raw_mod_map.len() / 8,
                map: raw_mod_map,
            }
        }
    }

    fn send_string(&mut self, s: &str) {
        for c in s.chars() {
            let keysym = char_to_keysym(c);
            let mapping = self.get_or_add_mapping(keysym).unwrap();
            self.send_keycode(mapping.keycode, mapping.modifiers);
        }
    }

    fn send_keycode(&self, keycode: u8, modifiers: ModMask) {
        for i in 0..8 {
            if modifiers.contains(1u16 << i) {
                let ev = KeyEvent{
                    press: true,
                    keycode: self.modifier_map[i][0]
                };
                fake_input(self.conn, ev);
            }
        }
        for press in [true, false] {
            fake_input(self.conn, KeyEvent{press, keycode});
        }
        for i in (0..8).rev() {
            if modifiers.contains(1u16 << i) {
                let ev = KeyEvent{
                    press: false,
                    keycode: self.modifier_map[i][0]
                };
                fake_input(self.conn, ev);
            }
        }
    }

    pub fn get_mapping(&self, keysym: u32) -> Option<Mapping> {
        self.keymap.get(&keysym).copied()
    }

    pub fn get_or_add_mapping(&mut self, keysym: u32) -> Option<Mapping> {
        let existing = self.keymap.get(&keysym).copied();
        if existing.is_some() { return existing; }
        if let Some(mut custom_mapping) = self.custom_mappings_queue.pop_back() {
            let previous_keysym = custom_mapping.keysym;
            self.keymap.remove(&previous_keysym);
            self.conn.change_keyboard_mapping(
                1,
                custom_mapping.keycode,
                1,
                &[keysym],
            ).unwrap().check().unwrap();
            custom_mapping.keysym = keysym;
            self.keymap.insert(keysym, custom_mapping);
            self.custom_mappings_queue.push_front(custom_mapping);
            return Some(custom_mapping);
        } else {
            return None;
        }
    }
}

fn is_latin1(code: char) -> bool
{
    let code = code as u32;
    (0x20 <= code && code <= 0x7e) || (0xa0 <= code && code <= 0xff)
}

fn char_to_keysym(c: char) -> Keysym {
    if is_latin1(c) {
        return c as u32;
    }
    if c == '\t' {
        return sym_defs::XK_Tab;
    }
    if c == '\n' || c == '\r' {
        return sym_defs::XK_Return;
    }
    unicode_to_keysym::char_to_keysym_map(c).unwrap_or((c as u32) | 0x01000000)
}