use crate::*;

use std::sync::mpsc::Sender;

pub type OutChannel<'a> = &'a Sender<Report>;
pub type FilterFn<'a> = &'a dyn Fn(State, OutChannel) -> State;

pub fn direct_passthrough(mut state: State, writer: OutChannel) -> State {
    let ev = match state.pressed.last() {
        Some(e) => e,
        None => { return state; }
    };

    let event_type = ev.event_type;
    let keycode = ev.usb_keycode;

    if event_type == EventType::KeyDown {
        state.pressed.retain(|x| x.usb_keycode != keycode);
    }

    let mut report = Report::new();
    for press in &state.pressed {
        report.add_key(press.usb_keycode);
    }

    writer.send(report).unwrap();

    state
}
