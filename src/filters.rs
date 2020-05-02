use crate::*;

use std::sync::mpsc::Sender;
use std::time::SystemTime;

use evdev::Key::*;
use log::*;

pub type OutChannel<'a> = &'a Sender<Report>;
pub type FilterFn<'a> = &'a dyn Fn(&mut State, OutChannel);

pub fn chording(state: &mut State, writer: OutChannel) {
    const MAX_WAIT_USECS: usize = 10;
    const MAX_CHORD_KEYS: usize = 2;

    let (pressed, mut hist) = state.view();
    let chord_keys = [KEY_UP, KEY_LEFT, KEY_RIGHT, KEY_DOWN];
    let chord_keys: Vec<UsbKeycode> = chord_keys.map(UsbKeycode::from).collect();

    let mut pressed_chord_keys = Vec::new();

    for ev in hist.iter_mut().rev() {
        let key = ev.usb_keycode();
        if chord_keys.contains(&key) {
            ev.handle();
            if ev.pressed() {
                pressed_chord_keys.push(key);
            }
        }
    }

    let len = pressed_chord_keys.len();
    if len > 1 {
        debug!("{} chord keys pressed simultaneously", len);
    }
}

pub fn direct_passthrough(state: &mut State, writer: OutChannel) {
    trace!("{:?}", state);
    let (pressed, mut hist) = state.view();

    for ev in hist.iter_mut().rev() {
        let keycode = ev.usb_keycode();
        if ev.released() {
            ev.consume();
        } else {
            ev.handle();
            pressed.push(keycode);
        }
    }
    trace!("{:?}", state);
}

pub fn direct_report(state: &mut State, writer: OutChannel) {
    writer.send(state.report()).unwrap();
}
