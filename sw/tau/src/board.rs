use daisy_embassy::pins::*;
use embassy_stm32::Peripheral;
use embassy_stm32::adc::{Adc, AdcChannel, SampleTime};
use embassy_stm32::gpio::{AnyPin, Input, Level, Output, Pin, Pull, Speed};
use embassy_stm32::peripherals::ADC1;

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

pub struct AUX2<PIN> {
    pin_adc: PIN,
}

impl<PIN> AUX2<PIN>
where
    PIN: AdcChannel<ADC1>,
{
    pub fn new(pin_adc: PIN) -> Self {
        Self { pin_adc }
    }

    pub fn read_raw(&mut self, adc: &mut Adc<'static, ADC1>) -> u16 {
        adc.blocking_read(&mut self.pin_adc)
    }

    pub fn read_f32(&mut self, adc: &mut Adc<'static, ADC1>) -> f32 {
        self.read_raw(adc) as f32 / 65535.0
    }
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
