#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Report {
    INPUT_DATA                  = 0x09,
    SET_MAPPINGS                = 0x80,
    CLEAR_MAPPINGS              = 0x81,
    GET_MAPPINGS                = 0x82,
    GET_ATTRIB                  = 0x83,
    GET_ATTRIB_LABEL            = 0x84,
    DEFAULT_MAPPINGS            = 0x85,
    FACTORY_RESET               = 0x86,
    WRITE_REGISTER              = 0x87,
    CLEAR_REGISTER              = 0x88,
    READ_REGISTER               = 0x89,
    GET_REGISTER_LABEL          = 0x8a,
    GET_REGISTER_MAX            = 0x8b,
    GET_REGISTER_DEFAULT        = 0x8c,
    SET_MODE                    = 0x8d,
    DEFAULT_MOUSE               = 0x8e,
    FORCE_FEEDBACK              = 0x8f,
    REQUEST_COMM_STATUS         = 0xb4,
    GET_SERIAL                  = 0xae,
    HAPTIC_PULSE                = 0xea
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
    for dev in api.device_list() {
        if dev.vendor_id() == 0x28de && dev.product_id() == 0x1205 && dev.interface_number() == 2 {
            found.push(dev);
        }
    }

    if found.len() == 0 {
        // todo: probably wait and retry
        panic!("No steam controller-deck found!");
    }

    if found.len() > 1 {
        panic!("Too many controllers! Which one...");
    }

    let dev = found.into_iter().next().unwrap().open_device(&api).unwrap();
    dbg!(&dev);
}
