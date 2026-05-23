use crate::STATE;
use crate::config;
use crate::midi::looper::{LooperControl, queue_looper_control};

use daisy_embassy::pins::*;
use defmt::info;
use embassy_stm32::Peripheral;
use embassy_stm32::adc::{Adc, AdcChannel, SampleTime};
use embassy_stm32::gpio::{AnyPin, Input, Level, Output, Pin, Pull, Speed};
use embassy_stm32::peripherals::ADC1;
use embassy_time::{Instant, Timer};

fn expression_raw_to_channel(raw: u16, current_channel: usize) -> usize {
    let raw = raw as u32;
    let current = current_channel.min(config::EXP_CHANNELS - 1);

    let lower = current as u32 * config::EXP_ZONE_WIDTH;
    let upper = (current as u32 + 1) * config::EXP_ZONE_WIDTH;

    if current < config::EXP_CHANNELS - 1 && raw >= upper + config::EXP_HYSTERESIS {
        (raw / config::EXP_ZONE_WIDTH).min((config::EXP_CHANNELS - 1) as u32) as usize
    } else if current > 0 && raw + config::EXP_HYSTERESIS < lower {
        (raw / config::EXP_ZONE_WIDTH).min((config::EXP_CHANNELS - 1) as u32) as usize
    } else {
        current
    }
}

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
        self.midi_channel = (self.midi_channel + 3) % 4;
    }

    pub fn next_channel(&mut self) {
        self.midi_channel = (self.midi_channel + 1) % 4;
    }

    pub fn set_channel(&mut self, channel: usize) {
        self.midi_channel = channel % 4;
    }

    pub fn channel(&self) -> usize {
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

pub type AppExpression = AUX2<SeedPin15>;

#[embassy_executor::task]
pub async fn aux_task(
    mut aux1: AUX1,
    mut exp: AppExpression,
    mut adc: Adc<'static, ADC1>,
    mut leds: ChLeds,
) {
    let mut sw2_pressed_at: Option<Instant> = None;
    let mut last_displayed_channel: Option<usize> = None;

    loop {
        // AUX1 footswitches -> looper control.
        if let Some(event) = aux1.poll_event() {
            match event {
                SwitchEvent::Sw1Pressed => {
                    info!("sw1 pressed");
                }

                SwitchEvent::Sw1Released => {
                    info!("sw1 released");
                    queue_looper_control(LooperControl::Sw1);
                }

                SwitchEvent::Sw2Pressed => {
                    info!("sw2 pressed");
                    sw2_pressed_at = Some(Instant::now());
                }

                SwitchEvent::Sw2Released => {
                    info!("sw2 released");

                    let held_ms = sw2_pressed_at
                        .map(|t| Instant::now().duration_since(t).as_millis())
                        .unwrap_or(0);

                    sw2_pressed_at = None;

                    info!("sw2 held_ms={}", held_ms);

                    if held_ms >= 1000 {
                        queue_looper_control(LooperControl::Clear);
                    } else {
                        queue_looper_control(LooperControl::PauseResume);
                    }
                }
            }
        }

        // AUX2 expression pedal -> MIDI channel select.
        let raw = exp.read_raw(&mut adc);

        let channel = {
            let mut state = STATE.lock().await;

            let current_channel = state.channel();
            let new_channel = expression_raw_to_channel(raw, current_channel);

            if new_channel != current_channel {
                state.set_channel(new_channel);
                info!("expression channel raw={} channel={}", raw, new_channel + 1);
            }

            state.channel()
        };

        if last_displayed_channel != Some(channel) {
            leds.set_channel(channel);
            last_displayed_channel = Some(channel);
        }

        Timer::after_millis(10).await;
    }
}
