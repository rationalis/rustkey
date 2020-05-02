use crate::*;

use getset::{Getters, CopyGetters};

pub type StateView<'a> = (&'a mut Vec<UsbKeycode>, Vec<&'a mut PressEvent>);

#[derive(Clone, Debug, Getters, CopyGetters)]
pub struct PressEvent {
    pub sim_keycode: UsbKeycode,

    #[getset(get_copy = "pub")]
    usb_keycode: UsbKeycode,

    #[getset(get_copy = "pub")]
    keydown_time: SystemTime,

    pub keyup_time: Option<SystemTime>,

    #[getset(get_copy = "pub")]
    status: PressStatus
}

/// This denotes the status of a `PressEvent` for a given frame (i.e. immediately
/// after the most recent `PressEvent`). If a `PressEvent` has been consumed,
/// then it should never be rehandled in the future. If a `PressEvent` has been
/// handled, then it should not be rehandled for the current frame. If a
/// `PressEvent` is unhandled then it will be handled by the end of the frame.
/// `State` and the default pipeline respects these conditions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PressStatus {
    Consumed,
    Handled,
    Unhandled
}

use PressStatus::*;

impl PressEvent {
    pub fn new(usb_keycode: UsbKeycode, time: SystemTime) -> Self {

        PressEvent {
            usb_keycode,
            sim_keycode: usb_keycode,
            keydown_time: time,
            keyup_time: None,
            status: Unhandled
        }
    }

    pub fn consume(&mut self) {
        debug_assert!(self.status == Unhandled);
        self.status = Consumed;
    }

    pub fn handle(&mut self) {
        debug_assert!(self.status == Unhandled);
        self.status = Handled;
    }

    pub fn pressed(&self) -> bool {
        self.keyup_time.is_none()
    }

    pub fn released(&self) -> bool {
        self.keyup_time.is_some()
    }
}

#[derive(Debug, Default)]
pub struct State {
    pub history: Vec<PressEvent>,
    pub pressed: Vec<UsbKeycode>
}

//pub type Predicate = impl FnMut(&&mut PressEvent) -> bool;

impl State {

    pub fn matcher(
        key: Option<Vec<UsbKeycode>>,
        released: Option<bool>,
        pressed_before: Option<SystemTime>,
        extends_past: Option<SystemTime>) ->
    impl FnMut(&&mut PressEvent) -> bool
    {
        move |e| {
            let key_match =
                match (key.as_ref(), e.usb_keycode()) {
                    (None, _) => true,
                    (Some(ks), k) => ks.contains(&k),
                    _ => false
                };

            let released_match =
                match (released, e.keyup_time) {
                    (None, _)
                        | (Some(true), Some(_))
                        | (Some(false), None) => true,
                    _ => false
                };

            let pressed_before_match =
                match pressed_before {
                    None => true,
                    Some(t) => e.keydown_time() <= t
                };

            let extends_past_match =
                match (extends_past, e.keyup_time) {
                    (None, _) => true,
                    (Some(t), Some(t2)) => t2 >= t,
                    (Some(_), None) => false // TODO: should compare against now
                };

            key_match
                && released_match
                && pressed_before_match
                && extends_past_match
        }
    }

    pub fn push(&mut self, key: UsbKeycode, ev_type: EventType,
                time: SystemTime) {
        match ev_type {
            EventType::KeyDown => {
                self.history.push(PressEvent::new(key, time));
            }
            EventType::KeyUp => {
                let ev =
                    self.history.iter_mut().rev().find(Self::matcher(
                        Some(vec![key]),
                        Some(false),
                        None,
                        None
                    )).unwrap();
                ev.keyup_time = Some(time);
            }
        }
    }

    pub fn reset(&mut self) {
        let mut all_consumed = true;
        for ev in self.history.iter_mut() {
            if ev.status != Consumed {
                all_consumed = false;
            }
            if ev.status == Handled {
                ev.status = Unhandled;
            }
        }

        if all_consumed {
            self.history.clear();
        }

        self.pressed.clear();
    }

    pub fn view(&mut self) -> StateView {
        (&mut self.pressed,
         self.history.iter_mut().filter(|ev| ev.status == Unhandled).collect())
    }

    pub fn report(&mut self) -> Report {
        let mut report = Report::new();
        for key in &self.pressed {
            report.add_key(*key);
        }
        self.reset();
        report
    }
}
