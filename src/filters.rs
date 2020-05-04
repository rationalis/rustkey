use crate::*;

use std::sync::mpsc::Sender;

use evdev::Key::*;

pub type OutChannel<'a> = &'a Sender<Report>;
pub type FilterFn = Box<dyn FnMut(&mut State, OutChannel)>;

pub fn relaxed_chording(state: &mut State, _writer: OutChannel) {
    const MAX_WAIT_MSECS: u64 = 4;
    const MAX_WAIT: Duration = Duration::from_millis(MAX_WAIT_MSECS);
    // TODO
    // const MAX_CHORD_KEYS: usize = 2;

    let (pressed, mut hist) = state.view();
    let chord_keys = [KEY_UP, KEY_LEFT, KEY_RIGHT, KEY_DOWN];
    let chord_keys: Vec<UsbKeycode> = chord_keys.iter().map(UsbKeycode::from).collect();

    let mut pressed_ck: Vec<&mut PressEvent> = Vec::new();
    let mut released_ck: Vec<&mut PressEvent> = Vec::new();

    // History is in order of oldest pressed first.
    for ev in hist.iter_mut() {
        if chord_keys.contains(&ev.usb_keycode()) {
            ev.handle();
            if ev.pressed() {
                pressed_ck.push(ev);
            } else {
                released_ck.push(ev);
            }
        }
    }

    for ev in released_ck.iter_mut() {
        if ev.keyup_time.unwrap().elapsed().unwrap() > MAX_WAIT {
            trace!("Wait time elapsed, sending single key");
            ev.consume();
            pressed.push(ev.usb_keycode());
            return;
        }
    }

    let len = pressed_ck.len();
    if len > 1 {
        trace!("{} chord keys pressed simultaneously", len);
        for ev in pressed_ck {
            ev.consume();
            pressed.push(ev.usb_keycode());
        }
        return;
    }

    let len2 = released_ck.len();
    if len + len2 > 1 {
        trace!("{} chord keys pressed within wait time", len + len2);
        for ev in pressed_ck.iter_mut().chain(released_ck.iter_mut()) {
            ev.consume();
            pressed.push(ev.usb_keycode());
        }
        return;
    }
}

pub fn direct_passthrough(state: &mut State, _writer: OutChannel) {
    trace!("{:?}", state);
    let (pressed, mut hist) = state.view();

    for ev in hist.iter_mut().rev() {
        if ev.released() {
            ev.consume();
        } else {
            ev.handle();
            pressed.push(ev.sim_keycode);
        }
    }
    trace!("{:?}", state);
}

pub fn direct_report(state: &mut State, writer: OutChannel) {
    writer.send(state.report()).unwrap();
}
