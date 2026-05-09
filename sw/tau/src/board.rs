use embassy_stm32::Peripheral;
use embassy_stm32::gpio::{AnyPin, Input, Level, Output, Pin, Pull, Speed};

pub struct ChLeds {
    ch1_led: Output<'static>,
    ch2_led: Output<'static>,
    ch3_led: Output<'static>,
    ch4_led: Output<'static>,
}

pub struct MIDIEnable {
    midi_enable: Input<'static>,
}

pub struct AUX1 {
    sw1: Input<'static>,
    sw2: Input<'static>,
}

pub struct AUX2 {
    pin_adc: Input<'static>,
}

impl ChLeds {
    pub fn new(
        ch1: impl Peripheral<P = impl Pin> + 'static,
        ch2: impl Peripheral<P = impl Pin> + 'static,
        ch3: impl Peripheral<P = impl Pin> + 'static,
        ch4: impl Peripheral<P = impl Pin> + 'static,
    ) -> Self {
        Self {
            ch1_led: Output::new(ch1, Level::Low, Speed::Medium),
            ch2_led: Output::new(ch2, Level::Low, Speed::Medium),
            ch3_led: Output::new(ch3, Level::Low, Speed::Medium),
            ch4_led: Output::new(ch4, Level::Low, Speed::Medium),
        }
    }

    pub fn set_channel(&mut self, channel: usize) {
        self.ch1_led.set_low();
        self.ch2_led.set_low();
        self.ch3_led.set_low();
        self.ch4_led.set_low();

        match channel {
            0 => self.ch1_led.set_high(),
            1 => self.ch2_led.set_high(),
            2 => self.ch3_led.set_high(),
            3 => self.ch4_led.set_high(),
            _ => {}
        }
    }
}

impl MIDIEnable {
    pub fn new(sw: impl Peripheral<P = impl Pin> + 'static) -> Self {
        Self {
            midi_enable: Input::new(sw, Pull::Up),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.midi_enable.is_low()
    }
}
