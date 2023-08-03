# Padtype

Padtype is a usermode program to allow typing with the steam deck's buttons and joysticks in linux. It uses hidapi and reverse engineerig from the [open-sd](https://gitlab.com/open-sd) project and uinput to simulate a keyboard. Because it shows as a regular keyboard device it should work everywhere a USB keyboard does.

Inspired by the "daisy wheel" keyboard in Steam Big Picture that was the perfect input method for typing with a gamepad and then removed (I'm still bitter), each combination of 8 directions on the joystick and each of the 4 buttons on the other side (left joystick+A,B,X,Y or right joystick+dpad) maps to a key, and L1, L2, R1, and R2 map to Alt, Ctrl, Shift, and Meta (aka Super aka Windows key aka Command) respectively.

This is meant to *replace* the steam client. If you run both they will both try to emulate a keyboard and you won't be happy.

## Todo

* Make the left trackpad scroll (right now it does nothing)
* Fix keys sometimes double-pressing
* Remove dependency on libudev.so (systemd) so that this can be staticly compiled
* Implement a mode switch to simulate a gamepad so you can play games yay

## Build

On most linux systems it's as simple as [installing rust](https://rustup.rs) and some `libudev` package, and then running `cargo build`.

On the steam deck `libudev` does not come installed, nor pkgconfig. This is a TODO but for now I recomend [installing nix on the steam deck](https://determinate.systems/posts/nix-on-the-steam-deck#an-invitation-to-experiment) and then using the `run` script.
