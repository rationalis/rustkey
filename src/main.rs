use std::convert::TryInto;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ctrlc;
use lazy_static::lazy_static;
use simplelog::*;

mod datatypes;
mod filters;

use datatypes::*;
use filters::*;

// https://github.com/torvalds/linux/blob/master/drivers/hid/usbhid/usbkbd.c
const USB_KBD_KEYCODE: [u8; 256] = [
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
	150,158,159,128,136,177,178,176,142,152,173,140,  0,  0,  0,  0
];

lazy_static!(
    static ref KEYCODE_MAP: [u8; 256] = reverse_map(&USB_KBD_KEYCODE);
);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventType {
    KeyDown,
    KeyUp
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UsbKeycode {
    data: u8
}

pub struct Report {
    mod_byte: u8,
    keys: [UsbKeycode; 6]
}

const NULL_KEY: UsbKeycode = UsbKeycode { data: 0 };

impl Report {
    const fn new() -> Report {
        Report::single_key(NULL_KEY)
    }

    const fn single_key(key: UsbKeycode) -> Report {
        let keys = [key, NULL_KEY, NULL_KEY, NULL_KEY, NULL_KEY, NULL_KEY];
        Report {
            mod_byte: 0,
            keys
        }
    }

    fn set_modifier(&mut self, m: &UsbKeycode) {
        match m.data {
            224 | 228 => self.mod_byte |= 1 << 0,
            225 | 229 => self.mod_byte |= 1 << 1,
            226 | 230 => self.mod_byte |= 1 << 2,
            227 | 231 => self.mod_byte |= 1 << 3,
            // TODO: handle win(gui) taps; it's dual role by default on windows
            // TODO: handle holding alt which is needed for alt-tabbing
            _ => panic!("Unrecognized modifier")
        }
    }

    fn add_key(&mut self, key: UsbKeycode) {
        if key.is_modifier() {
            self.set_modifier(&key);
            return;
        }

        for i in 0..6 {
            if self.keys[i].data == 0 {
                self.keys[i] = key;
                return;
            }
        }

        panic!("Exceeded 6KRO")
    }

    fn data(&self) -> [u8; 8] {
        let mut report = [0; 8];
        report[0] = self.mod_byte;
        for i in 0..6 {
            report[i+2] = self.keys[i].data;
        }
        report
    }
}

impl UsbKeycode {
    fn from_evdev_code(ev: &evdev::raw::input_event) -> UsbKeycode {
        let ev_code: usize = ev.code.try_into().unwrap();
        UsbKeycode {
            data: KEYCODE_MAP[ev_code]
        }
    }

    fn is_modifier(&self) -> bool {
        self.data >= 224 && self.data <= 231
    }
}

fn reverse_map(arr: &[u8]) -> [u8; 256] {
    let mut map: [u8; 256] = [0; 256];
    for (i,j) in arr.iter().enumerate() {
        map[*j as usize] = i as u8;
    }
    map
}

fn main() {
    let _ = SimpleLogger::init(LevelFilter::Debug, Config::default()).unwrap();

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

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).unwrap();

    let _writer = thread::spawn(move || {
        let mut out: File =
            OpenOptions::new().write(true).open("/dev/hidg0").unwrap();
        loop {
            let r = writer_receiver.recv();
            if r.is_err() {
                break;
            }
            let r: Report = r.unwrap();
            out.write(&r.data()).unwrap();
        }
    });

    let manager = thread::spawn(move || {
        use std::sync::mpsc::RecvTimeoutError::*;
        let mut state: State = State::default();
        while running.load(Ordering::SeqCst) {
            let ev = manager_receiver.recv_timeout(Duration::from_millis(10));
            let ev = match ev {
                Err(Timeout) => { continue; }
                Err(Disconnected) => { break; }
                Ok(e) => e
            };

            if ev._type == 1 {
                // let was_empty = state.pressed.is_empty();
                let usb_keycode = UsbKeycode::from_evdev_code(&ev);
                if ev.value == 0 || ev.value == 1 {
                    let secs = ev.time.tv_sec;
                    let usecs = ev.time.tv_usec;
                    let time = UNIX_EPOCH
                        + Duration::from_secs(secs.try_into().unwrap())
                        + Duration::from_micros(usecs.try_into().unwrap());

                    // This typically measured ~100us +/- 50us of delay between
                    // the time at which this thread reads the event, and the
                    // timestamp evdev attaches to it. While this seems quite
                    // unlikely to affect anything, in principle using the
                    // evdev timestamp is more precise.

                    // println!("{:?}", SystemTime::now().duration_since(time).unwrap());
                    state.push(PressEvent::new(
                        usb_keycode,
                        match ev.value {
                            0 => EventType::KeyUp,
                            1 => EventType::KeyDown,
                            _ => unreachable!()
                        },
                        time));

                    chording(&mut state, &to_writer);
                    direct_passthrough(&mut state, &to_writer);
                    direct_report(&mut state, &to_writer);
                }
            }
        }
        to_writer.send(Report::new()).unwrap();
    });

    let _reader = thread::spawn(move || {
        let mut prev = SystemTime::now();
        // let mut accum = Duration::new(0, 0);
        // let mut counter = 0;
        'main: loop {
            let mut sent_something = false;
            for ev in d.events_no_sync().unwrap() {
                // println!("{:?}", ev);
                // forward(&ev);
                let res = to_manager.send(ev);
                if res.is_err() {
                    break 'main;
                }
                sent_something = true;
            }
            let next = SystemTime::now();
            let duration = next.duration_since(prev).unwrap();
            if sent_something {
                // println!("\n{:?}", duration);
            }
            // accum += duration;
            // counter += 1;
            // println!("{:?}", accum / counter);
            prev = next;
        }
    });

    manager.join().unwrap();
}
