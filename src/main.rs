use std::convert::TryInto;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
//use std::sync::Mutex;
//use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use lazy_static::lazy_static;

// https://github.com/torvalds/linux/blob/master/drivers/hid/usbhid/usbkbd.c
const USB_KBD_KEYCODE: [u8; 252] = [
	  0,  0,  0,  0, 30, 48, 46, 32, 18, 33, 34, 35, 23, 36, 37, 38,
	 50, 49, 24, 25, 16, 19, 31, 20, 22, 47, 17, 45, 21, 44,  2,  3,
	  4,  5,  6,  7,  8,  9, 10, 11, 28,  1, 14, 15, 57, 12, 13, 26,
	 27, 43, 43, 39, 40, 41, 51, 52, 53, 58, 59, 60, 61, 62, 63, 64,
	 65, 66, 67, 68, 87, 88, 99, 70,119,110,102,104,111,107,109,106,
	105,108,103, 69, 98, 55, 74, 78, 96, 79, 80, 81, 75, 76, 77, 71,
	 72, 73, 82, 83, 86,127,116,117,183,184,185,186,187,188,189,190,
	191,192,193,194,134,138,130,132,128,129,131,137,133,135,136,113,
	115,114,  0,  0,  0,121,  0, 89, 93,124, 92, 94, 95,  0,  0,  0,
	122,123, 90, 91, 85,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
	  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
	  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
	  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
	  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
	 29, 42, 56,125, 97, 54,100,126,164,166,165,163,161,115,114,113,
	150,158,159,128,136,177,178,176,142,152,173,140
];

lazy_static!(
    static ref KEYCODE_MAP: [u8; 256] = reverse_map(&USB_KBD_KEYCODE);
);

// TODO: replace random arbitrary u8 with UsbKeycode where possible
struct UsbKeycode {
    data: u8
}

struct Report {
    data: [u8; 8]
}

const NULL_REPORT: Report = Report { data: [0,0,0,0,0,0,0,0] };

struct PressEvent {
    usb_keycode: u8,
    time: Instant
}

struct State {
    pressed: Vec<PressEvent>
}

impl UsbKeycode {
    fn from_evdev_code(ev: &evdev::raw::input_event) -> UsbKeycode {
        let ev_code: usize = ev.code.try_into().unwrap();
        UsbKeycode {
            data: KEYCODE_MAP[ev_code]
        }
    }
}

fn char_report(c: u8) -> Report {
    Report {
        data: [0, 0, c, 0, 0, 0, 0, 0]
    }
}

fn reverse_map(arr: &[u8]) -> [u8; 256] {
    let mut map: [u8; 256] = [0; 256];
    for (i,j) in arr.iter().enumerate() {
        map[*j as usize] = i as u8;
    }
    map
}

fn is_modifier(c: u8) -> bool {
    c >= 224 && c <= 231
}

fn modify(c: &mut Report, m: u8) {
    match m {
        224 | 228 => c.data[0] |= 1 << 0,
        225 | 229 => c.data[0] |= 1 << 1,
        226 | 230 => c.data[0] |= 1 << 2,
        227 | 231 => c.data[0] |= 1 << 3,
        // TODO: handle win(gui) taps; it's dual role by default on windows
        // TODO: handle holding alt which is needed for alt-tabbing
        _ => panic!()
    }
}

fn main() {
    let mut args = std::env::args_os();
    let mut d;
    if args.len() > 1 {
        d = evdev::Device::open(&args.nth(1).unwrap()).unwrap();
    } else {
        let mut devices = evdev::enumerate();
        for (i, d) in devices.iter().enumerate() {
            println!("{}: {:?}", i, d.name());
        }
        print!("Select the device [0-{}]: ", devices.len());
        let _ = std::io::stdout().flush();
        let mut chosen = String::new();
        std::io::stdin().read_line(&mut chosen).unwrap();
        d = devices.swap_remove(chosen.trim().parse::<usize>().unwrap());
    }
    println!("{}", d);
    println!("Events:");

    let (to_manager, manager_receiver) = mpsc::channel::<evdev::raw::input_event>();
    let (to_writer, writer_receiver) = mpsc::channel::<Report>();

    let writer = thread::spawn(move || {
        let mut out: File =
            OpenOptions::new().write(true).open("/dev/hidg0").unwrap();
        loop {
            let r: Report = writer_receiver.recv().unwrap();
            out.write(&r.data).unwrap();
        }
    });

    let manager = thread::spawn(move || {
        let mut state: State = State { pressed: Vec::new() };
        loop {
            let ev = manager_receiver.recv().unwrap();
            if ev._type == 1 {
                let keycode: usize = ev.code.try_into().unwrap();
                let usb_keycode = KEYCODE_MAP[keycode];
                if usb_keycode >= 224 {
                    // modifier keys
                    if ev.value == 1 {
                        // Note that this will record the time that the manager
                        // thread processes a received event, as opposed to the
                        // more accurate time at which the reader thread sends
                        // the event.
                        // TODO: attach time info in reader
                        state.pressed.push(PressEvent{
                            usb_keycode,
                            time: Instant::now()
                        });
                    } else if ev.value == 0 {
                        state.pressed.retain(|x| x.usb_keycode != usb_keycode);
                    }
                }
                else if usb_keycode < 224 && ev.value == 1 {
                    let mut c = char_report(usb_keycode);
                    for press in &state.pressed {
                        if is_modifier(press.usb_keycode) {
                            modify(&mut c, press.usb_keycode);
                        }
                    }
                    to_writer.send(c).unwrap();
                    to_writer.send(NULL_REPORT).unwrap();
                }
            }
        }
    });

    let reader = thread::spawn(move || {
        let mut prev = Instant::now();
        // let mut accum = Duration::new(0, 0);
        // let mut counter = 0;
        loop {
            let mut sent_something = false;
            for ev in d.events_no_sync().unwrap() {
                // println!("{:?}", ev);
                // forward(&ev);
                to_manager.send(ev).unwrap();
                sent_something = true;
            }
            let next = Instant::now();
            let duration = next - prev;
            if sent_something {
                // println!("\n{:?}", duration);
            }
            // accum += duration;
            // counter += 1;
            // println!("{:?}", accum / counter);
            prev = next;
        }
    });

    writer.join().unwrap();
    manager.join().unwrap();
    reader.join().unwrap();
}
