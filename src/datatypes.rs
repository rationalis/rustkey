use crate::*;

use getset::{Getters, CopyGetters};

pub type StateView<'a> = (&'a mut Vec<UsbKeycode>, Vec<&'a mut PressEvent>);

#[derive(Clone, Debug, Getters, CopyGetters)]
pub struct PressEvent {
    pub sim_keycode: UsbKeycode,

    #[getset(get_copy = "pub")]
    usb_keycode: UsbKeycode,

    #[getset(get_copy = "pub")]
    event_type: EventType,

    #[getset(get_copy = "pub")]
    time: SystemTime,

    #[getset(get_copy = "pub")]
    status: PressStatus
}

/// This denotes the status of a `PressEvent` for a given frame (i.e. immediately
/// after the most recent `PressEvent`). If a `PressEvent` has been consumed,
/// then it should never be rehandled in the future. If a `PressEvent` has been
/// handled, then it should not be rehandled for a current frame. If a
/// `PressEvent` is unhandled then it should be handled by the end of the frame.
/// `State` and the default pipeline respects these conditions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PressStatus {
    Consumed,
    Handled,
    Unhandled
}

use PressStatus::*;

impl PressEvent {
    pub fn new(usb_keycode: UsbKeycode, event_type: EventType, time: SystemTime)
               -> Self {

        PressEvent {
            usb_keycode,
            sim_keycode: usb_keycode,
            event_type,
            time,
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
}

#[derive(Debug, Default)]
pub struct State {
    pub history: Vec<PressEvent>,
    pub pressed: Vec<UsbKeycode>
}

impl State {
    pub fn push(&mut self, ev: PressEvent) {
        self.history.push(ev);
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
