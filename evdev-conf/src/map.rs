use evdev::Key;

pub const LEFT_STICK_KEYS:&[Key] = &[
    Key::KEY_TAB,
    Key::KEY_SPACE,
    Key::KEY_ENTER,
    Key::KEY_BACKSPACE,

    Key::KEY_A,
    Key::KEY_B,
    Key::KEY_C,
    Key::KEY_D,

    Key::KEY_E,
    Key::KEY_F,
    Key::KEY_G,
    Key::KEY_H,

    Key::KEY_I,
    Key::KEY_J,
    Key::KEY_K,
    Key::KEY_L,

    Key::KEY_M,
    Key::KEY_N,
    Key::KEY_O,
    Key::KEY_P,

    Key::KEY_Q,
    Key::KEY_R,
    Key::KEY_S,
    Key::KEY_T,

    Key::KEY_U,
    Key::KEY_V,
    Key::KEY_W,
    Key::KEY_X,

    // yz;/,.\'
    Key::KEY_Y,
    Key::KEY_Z,
    Key::KEY_SEMICOLON,
    Key::KEY_SLASH,

    Key::KEY_COMMA,
    Key::KEY_DOT,
    Key::KEY_BACKSLASH,
    Key::KEY_APOSTROPHE,
];

pub const RIGHT_STICK_KEYS:&[Key] = &[
    Key::KEY_UP,
    Key::KEY_RIGHT,
    Key::KEY_DOWN,
    Key::KEY_LEFT,

    Key::KEY_1,
    Key::KEY_2,
    Key::KEY_3,
    Key::KEY_4,

    Key::KEY_5,
    Key::KEY_6,
    Key::KEY_7,
    Key::KEY_8,
    
    Key::KEY_9,
    Key::KEY_0,
    Key::KEY_MINUS,
    Key::KEY_EQUAL,

    Key::KEY_LEFTBRACE,
    Key::KEY_RIGHTBRACE,
    Key::KEY_GRAVE,
    Key::KEY_F19,

    Key::KEY_ESC,
    Key::KEY_PRINT,
    Key::KEY_INSERT,
    Key::KEY_DELETE,

    Key::KEY_F1,
    Key::KEY_F2,
    Key::KEY_F3,
    Key::KEY_F4,

    Key::KEY_F5,
    Key::KEY_F6,
    Key::KEY_F7,
    Key::KEY_F8,

    Key::KEY_F9,
    Key::KEY_F10,
    Key::KEY_F11,
    Key::KEY_F12,

];