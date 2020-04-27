use crate::*;

use std::sync::mpsc::Sender;

pub type OutChannel<'a> = &'a Sender<Report>;
pub type FilterFn<'a> = &'a dyn Fn(&mut State, OutChannel);

pub fn direct_passthrough(state: &mut State, writer: OutChannel) {
    //println!("{:?}", state);
    let (pressed, mut hist) = state.view();
    let mut released = Vec::new();

    for ev in hist.iter_mut().rev() {
        use EventType::*;
        let keycode = ev.usb_keycode();
        match ev.event_type() {
            KeyUp => {
                ev.consume();
                released.push(keycode);
            }
            KeyDown => {
                if released.contains(&keycode) {
                    ev.consume();
                } else {
                    ev.handle();
                    pressed.push(keycode);
                }
            }
        }
    }
    //println!("{:?}", state);
}

pub fn direct_report(state: &mut State, writer: OutChannel) {
    writer.send(state.report()).unwrap();
}
