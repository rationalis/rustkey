`rustkey` is meant to address several limitations or inconveniences involved with trying to retrofit programmability on an
existing, non-programmable keyboard. It is meant be run in the background on a Linux machine which acts as a proxy between
a keyboard and a PC (both connected via USB).

`rustkey` is written in Rust as a project to explore the usage of Rust for safe, high-level behavior interacting with
unsafe, low-level components. It uses evdev and the Linux USB gadget drivers to receive input from the keyboard and send
USB output to the PC emulating a keyboard.

In other words, the purpose of `rustkey` is to act as an external microcontroller for a keyboard, allowing any
keyboard to be fully programmable in terms of its USB inputs and outputs. Currently it is being developed and tested on a
Raspberry Pi 4, which has 2 separate USB buses allowing it to function as both a USB host for the keyboard and a USB device
for the connected PC.

Alternatives / existing solutions:

- Replacing the keyboard microcontroller
- Attach an external microcontroller e.g. via [Haku USB-to-USB converter](https://www.1upkeyboards.com/shop/controllers/usb-to-usb-converter/)
- Keyboard macro / remapping programs e.g. AutoHotkey

Compared to these, `rustkey` has a unique set of benefits:
- No specialized hardware or hardware modifications required
- Effectively unlimited memory for keyboard layouts
- More programmability and connectivity
- No need to re-flash ROM to reprogram
- Extensible to any USB inputs / outputs which have the appropriate Linux drivers available
- Platform-independent functionality, e.g. on Windows:
  - Intercepting reserved key chords like `Win+L` or `Ctrl+Alt+Del`
  - Distinguishing between multiple connected keyboards
- Macros which are totally transparent and undetectable to the host:
  - Allows macros to continue functioning in the presence of game anti-cheat programs
  - Allows a hardware-secured method (assuming no network connectivity) for password management

Thus `rustkey` enables vastly more programmability and extensibility than other existing solutions, with minimal, software-only,
keyboard/OS-independent setup, and only commodity hardware. While applicable only to a small niche, it is intended to be an extremely
useful tool for that niche, enabling transparent, multi-keyboard, cross-platform macros, which can trigger arbitrarily complex behavior.

There are some disadvantages:
- Unlike a traditional microcontroller, the controller Linux PC cannot directly access keyboard internals. While it is certainly
possible to do so with additional setup, raw analog IO is out of scope for the foreseeable future.
- Much greater power consumption, compared to simple microcontrollers.
- Any behavior not directly tied to direct keyboard IO, such as detecting the currently focused window, cannot be accessed without
additional setup, unlike platform-specific software tools.
