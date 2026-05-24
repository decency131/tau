# tau

![Rust](https://img.shields.io/badge/-Rust-orange?style=for-the-badge&logo=rust&logoColor=white)

tau is a pedal that converts musical instrument input into USB MIDI notes.

It is based on [daisy seed by Electrosmith](https://daisy.audio/hardware/Seed/) and [daisy-embassy](https://github.com/daisy-embassy/daisy-embassy) (which is based on [embassy](https://embassy.dev/), an embedded async runtime).

Current main branch contains firmware for the first revision hardware. Firmware for the second revision can be found on the [rev2-attempt branch](https://github.com/decency131/tau/tree/rev2-attempt) and is mostly finished, apart from optimization work currently present on the main branch. Second revision hardware needs some adjustments and fixes that are planned to be implemented in the future.

## Current features 

- Real-time pitch detection from instrument input
- USB MIDI output
- MIDI note on/off generation based on detected pitch
- Channel selection using an expression pedal
- Basic MIDI looper functionality
- Footswitch control:
  - SW1: record / play / overdub
  - SW2 short press: pause / resume
  - SW2 long press: clear loop
- Channel indicator LEDs

## How it works

The audio input is received from the Daisy Seed audio codec. Audio frames are copied from the audio callback into a separate pitch detection task, so heavier DSP work does not run directly inside the audio callback.

Pitch detection is done using a [no-std YIN implementation](https://github.com/decency131/yin-no_std). Detected frequencies are converted to MIDI notes. A small smoothing layer is used to reduce note flicker and avoid fast retriggering when the detected note is unstable.

MIDI events are sent over USB MIDI. The looper records MIDI events together with their timing and MIDI channel, so previously recorded loops keep playing on the original channel even if the expression pedal changes the current channel later.

## Build
```zsh
cargo build --features=seed_1_2 --release
```
Adjust to your seed revision.

## Flash/run

The recommended way is to use a debug probe (such as ST-LINK v3 minie), and run
```zsh 
cargo run --features=seed_1_2 --release
```
or 
```zsh 
DEFMT_LOG=warn cargo run --features=seed_1_2 --release
```
for less logging overhead.

Electrosmith have, however, recently [made their bootloader open source](https://github.com/electro-smith/DaisyBootloader), so you could try using that, or their [online flasher](https://flash.daisy.audio/).



> [!NOTE]
> This project is released under the **WTFPL** license.
