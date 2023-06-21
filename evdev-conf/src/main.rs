fn main() {
    for (p, d) in evdev::enumerate() {
        dbg!(p, d.name(), d.physical_path(), d.unique_name(), d.input_id(), d.properties(),
        d.supported_events(),
        d.supported_keys(),
        d.supported_relative_axes(),
        d.supported_absolute_axes(),
        d.supported_switches(),
        d.supported_ff(),
        );
    }
}
