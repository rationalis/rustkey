`rustkey` is meant to address several limitations or inconveniences involved with trying to retrofit programmability on an
existing, non-programmable keyboard. It is meant be run in the background on a Linux machine which is connected to a keyboard
and another PC via USB.

`rustkey` is written in Rust as a project to explore the usage of Rust for safe, high-level behavior interacting with
unsafe, low-level components. It uses evdev and the Linux USB gadget drivers to receive input from the keyboard and send
USB output to the PC emulating a keyboard.

In other words, the purpose of `rustkey` is to act as an external microcontroller for a keyboard, allowing any
keyboard to be fully programmable in terms of its USB inputs and outputs. Currently it is being developed and tested on a
Raspberry Pi 4, which has 2 separate USB buses allowing it to function as both a USB host for the keyboard and a USB device
for the connected PC.

This has several advantages:
- No specialized hardware or hardware modifications required, unlike existing solutions such as replacing the keyboard
microcontroller or the limited-production Haku USB-to-USB converter which inspired this project
- Effectively unlimited memory for keyboard layouts, compared to traditional microcontrollers
- More programmability and connectivity compared to traditional microcontrollers, e.g. being able to send and receive data
over a connected network in response to a key press
- No need to re-flash ROM to reprogram, compared to traditional microcontrollers
- Extensible to any USB inputs / outputs which have the appropriate Linux drivers available
- Platform-independent functionality, compared to non-programmable keyboard customization and/or OS-dependent macro programs
like AutoHotkey and its Linux analogues, including:
- On Windows, distinguishing between multiple connected keyboards, or intercepting special keys like `Win` or `Ctrl+Alt+Del`
- Macros which are totally transparent and undetectable to the host, which allow macros to continue functioning in the
presence of e.g. game anti-cheat programs, or act as a hardware-secured way (assuming no network connectivity) to store and
retrieve passwords

Thus `rustkey` enables vastly more extensive programmability and extensibility than other existing solutions, with
minimal, software-only, keyboard/OS-independent setup, and only commodity hardware. While applicable only to a small niche, it
is intended to be an extremely useful tool for that niche, enabling transparent, multi-keyboard, cross-platform macros, which
can trigger arbitrarily complex behavior.

There are some disadvantages:
- Unlike a traditional microcontroller, the controller Linux PC cannot directly access keyboard internals. While it is certainly
possible to do so, raw analog IO is out of scope for the foreseeable future.
- Vastly greater power consumption, compared to simple microcontrollers
- Any behavior not directly tied to direct keyboard IO, such as detecting the currently focused window, cannot be accessed without
additional setup, unlike AutoHotkey and other platform-specific software tools
