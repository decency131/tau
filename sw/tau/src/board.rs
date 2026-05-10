use crate::STATE;

use daisy_embassy::pins::*;
use defmt::info;
use embassy_stm32::Peripheral;
use embassy_stm32::adc::{Adc, AdcChannel, SampleTime};
use embassy_stm32::gpio::{AnyPin, Input, Level, Output, Pin, Pull, Speed};
use embassy_stm32::peripherals::ADC1;
use embassy_time::Timer;

#[derive(Clone, Copy)]
pub enum SwitchEvent {
    Sw1Pressed,
    Sw1Released,
    Sw2Pressed,
    Sw2Released,
}

pub struct State {
    midi_channel: usize,
}

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

    sw1_was_pressed: bool,
    sw2_was_pressed: bool,
}

pub struct AUX2<PIN> {
    pin_adc: PIN,
}

impl State {
    pub const fn new() -> Self {
        Self { midi_channel: 0 }
    }

    pub fn prev_channel(&mut self) {
        self.midi_channel = (self.midi_channel - 1) % 4;
    }

    pub fn next_channel(&mut self) {
        self.midi_channel = (self.midi_channel + 1) % 4;
    }

    pub fn channel(&mut self) -> usize {
        self.midi_channel
    }
}

impl AUX1 {
    pub fn new(
        sw1: impl Peripheral<P = impl Pin> + 'static,
        sw2: impl Peripheral<P = impl Pin> + 'static,
    ) -> Self {
        Self {
            sw1: Input::new(sw1, Pull::Up),
            sw2: Input::new(sw2, Pull::Up),

            sw1_was_pressed: false,
            sw2_was_pressed: false,
        }
    }

    pub fn sw1_pressed(&self) -> bool {
        self.sw1.is_low()
    }

    pub fn sw2_pressed(&self) -> bool {
        self.sw2.is_low()
    }

    pub fn sw1_just_pressed(&mut self) -> bool {
        let pressed = self.sw1_pressed();
        let just_pressed = pressed && !self.sw1_was_pressed;
        self.sw1_was_pressed = pressed;
        just_pressed
    }

    pub fn sw2_just_pressed(&mut self) -> bool {
        let pressed = self.sw2_pressed();
        let just_pressed = pressed && !self.sw2_was_pressed;
        self.sw2_was_pressed = pressed;
        just_pressed
    }

    pub fn poll_event(&mut self) -> Option<SwitchEvent> {
        let sw1_pressed = self.sw1_pressed();
        if sw1_pressed != self.sw1_was_pressed {
            self.sw1_was_pressed = sw1_pressed;

            return Some(if sw1_pressed {
                SwitchEvent::Sw1Pressed
            } else {
                SwitchEvent::Sw1Released
            });
        }

        let sw2_pressed = self.sw2_pressed();
        if sw2_pressed != self.sw2_was_pressed {
            self.sw2_was_pressed = sw2_pressed;

            return Some(if sw2_pressed {
                SwitchEvent::Sw2Pressed
            } else {
                SwitchEvent::Sw2Released
            });
        }

        None
    }
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

#[embassy_executor::task]
pub async fn aux_task(mut aux1: AUX1, mut leds: ChLeds) {
    loop {
        if let Some(event) = aux1.poll_event() {
            match event {
                SwitchEvent::Sw1Pressed => {
                    info!("sw1 pressed");
                }
                SwitchEvent::Sw1Released => {
                    info!("sw1 released");
                    let channel = {
                        let mut state = STATE.lock().await;
                        state.prev_channel();
                        state.channel()
                    };

                    leds.set_channel(channel);
                }
                SwitchEvent::Sw2Pressed => {
                    info!("sw2 pressed");
                }
                SwitchEvent::Sw2Released => {
                    info!("sw2 released");
                    let channel = {
                        let mut state = STATE.lock().await;
                        state.next_channel();
                        state.channel()
                    };

                    leds.set_channel(channel);
                }
            }
        }

        Timer::after_millis(10).await;
    }
}
