use core::str;

use crate::STATE;
use crate::colours::Rgb;
use crate::midi::looper::{LooperControl, queue_looper_control};

use daisy_embassy::pins::*;
use defmt::info;
use embassy_stm32::Peripheral;
use embassy_stm32::adc::{Adc, AdcChannel, SampleTime};
use embassy_stm32::gpio::{AnyPin, Input, Level, Output, Pin, Pull, Speed};
use embassy_stm32::peripherals::ADC1;
use embassy_time::{Instant, Timer};

const WS2812_T0H_CYCLES: u32 = 120;
const WS2812_T0L_CYCLES: u32 = 300;

const WS2812_T1H_CYCLES: u32 = 300;
const WS2812_T1L_CYCLES: u32 = 300;

const WS2812_RESET_US: u64 = 300;

fn mode_color(mode: AuxMode) -> Rgb {
    match mode {
        AuxMode::Off => Rgb::BLACK,
        AuxMode::FsLooper => Rgb::GREEN,
        AuxMode::FsChannel => Rgb::YELLOW,
        AuxMode::ExpModulation => Rgb::BLUE,
        AuxMode::ExpVelocity => Rgb::MAGENTA,
    }
}

async fn handle_aux_switch_event(
    aux_number: usize,
    mode: AuxMode,
    event: SwitchEvent,
    sw2_pressed_at: &mut Option<Instant>,
) {
    match mode {
        AuxMode::FsLooper => match event {
            SwitchEvent::Sw1Pressed => {
                info!("aux{} sw1 pressed", aux_number);
            }

            SwitchEvent::Sw1Released => {
                info!("aux{} sw1 released", aux_number);
                queue_looper_control(LooperControl::Sw1);
            }

            SwitchEvent::Sw2Pressed => {
                info!("aux{} sw2 pressed", aux_number);
                *sw2_pressed_at = Some(Instant::now());
            }

            SwitchEvent::Sw2Released => {
                info!("aux{} sw2 released", aux_number);

                let held_ms = sw2_pressed_at
                    .map(|t| Instant::now().duration_since(t).as_millis())
                    .unwrap_or(0);

                *sw2_pressed_at = None;

                info!("aux{} sw2 held_ms={}", aux_number, held_ms);

                if held_ms >= 1000 {
                    queue_looper_control(LooperControl::Clear);
                } else {
                    queue_looper_control(LooperControl::PauseResume);
                }
            }
        },

        AuxMode::FsChannel => match event {
            SwitchEvent::Sw1Released => {
                let channel = {
                    let mut state = STATE.lock().await;
                    state.prev_channel();
                    state.channel()
                };

                info!("aux{} prev channel={}", aux_number, channel);
            }

            SwitchEvent::Sw2Released => {
                let channel = {
                    let mut state = STATE.lock().await;
                    state.next_channel();
                    state.channel()
                };

                info!("aux{} next channel={}", aux_number, channel);
            }

            _ => {}
        },

        _ => {}
    }
}

async fn handle_aux_expression(aux_number: usize, mode: AuxMode, value: f32) {
    match mode {
        AuxMode::ExpModulation => {
            info!("aux{} modulation={}", aux_number, value);
        }

        AuxMode::ExpVelocity => {
            info!("aux{} velocity={}", aux_number, value);
        }

        _ => {}
    }
}

#[derive(Clone, Copy)]
pub enum SwitchEvent {
    Sw1Pressed,
    Sw1Released,
    Sw2Pressed,
    Sw2Released,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AuxMode {
    Off,
    FsLooper,
    FsChannel,
    ExpModulation,
    ExpVelocity,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AuxElectricalMode {
    Footswitch,
    Expression,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SetupState {
    Idle,
    Aux1,
    Aux2,
}

#[derive(Clone, Copy)]
pub enum EncoderEvent {
    Clockwise,
    CounterClockwise,
}

pub struct State {
    midi_channel: usize,

    aux1_mode: AuxMode,
    aux2_mode: AuxMode,

    setup_state: SetupState,
}

pub struct ChLeds {
    ch1_led: Output<'static>,
    ch2_led: Output<'static>,
    ch3_led: Output<'static>,
    ch4_led: Output<'static>,
}

pub struct Setup {
    enc_a: Input<'static>,
    enc_b: Input<'static>,
    enc_sw: Input<'static>,

    led_din: Output<'static>,

    last_a: bool,
    last_b: bool,
    sw_was_pressed: bool,
}

pub struct MIDIEnable {
    midi_enable: Input<'static>,
}

pub struct AuxPort<PIN> {
    sw1: Input<'static>,
    sw2: Input<'static>,
    adc: PIN,
    state_pin: Output<'static>,

    sw1_was_pressed: bool,
    sw2_was_pressed: bool,
}

pub struct AUX<ADC_PIN1, ADC_PIN2> {
    aux1: AuxPort<ADC_PIN1>,
    aux2: AuxPort<ADC_PIN2>,
}

impl AuxMode {
    pub fn electrical_mode(self) -> AuxElectricalMode {
        match self {
            AuxMode::Off => AuxElectricalMode::Footswitch,
            AuxMode::FsLooper => AuxElectricalMode::Footswitch,
            AuxMode::FsChannel => AuxElectricalMode::Footswitch,
            AuxMode::ExpModulation => AuxElectricalMode::Expression,
            AuxMode::ExpVelocity => AuxElectricalMode::Expression,
        }
    }

    pub fn is_footswitch(self) -> bool {
        matches!(self, AuxMode::FsLooper | AuxMode::FsChannel)
    }

    pub fn is_expression(self) -> bool {
        matches!(self, AuxMode::ExpModulation | AuxMode::ExpVelocity)
    }

    pub fn next(self) -> Self {
        match self {
            AuxMode::Off => AuxMode::FsLooper,
            AuxMode::FsLooper => AuxMode::FsChannel,
            AuxMode::FsChannel => AuxMode::ExpModulation,
            AuxMode::ExpModulation => AuxMode::ExpVelocity,
            AuxMode::ExpVelocity => AuxMode::Off,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            AuxMode::Off => AuxMode::ExpVelocity,
            AuxMode::FsLooper => AuxMode::Off,
            AuxMode::FsChannel => AuxMode::FsLooper,
            AuxMode::ExpModulation => AuxMode::FsChannel,
            AuxMode::ExpVelocity => AuxMode::ExpModulation,
        }
    }
}

impl State {
    pub const fn new() -> Self {
        Self {
            midi_channel: 0,
            aux1_mode: AuxMode::Off,
            aux2_mode: AuxMode::Off,
            setup_state: SetupState::Idle,
        }
    }

    pub fn prev_channel(&mut self) {
        self.midi_channel = (self.midi_channel + 3) % 4;
    }

    pub fn next_channel(&mut self) {
        self.midi_channel = (self.midi_channel + 1) % 4;
    }

    pub fn channel(&self) -> usize {
        self.midi_channel
    }

    pub fn aux1_mode(&self) -> AuxMode {
        self.aux1_mode
    }

    pub fn aux2_mode(&self) -> AuxMode {
        self.aux2_mode
    }

    pub fn set_aux1_mode(&mut self, mode: AuxMode) {
        self.aux1_mode = mode;
    }

    pub fn set_aux2_mode(&mut self, mode: AuxMode) {
        self.aux2_mode = mode;
    }

    pub fn next_aux1_mode(&mut self) {
        self.aux1_mode = self.aux1_mode.next();
    }

    pub fn prev_aux1_mode(&mut self) {
        self.aux1_mode = self.aux1_mode.prev();
    }

    pub fn next_aux2_mode(&mut self) {
        self.aux2_mode = self.aux2_mode.next();
    }

    pub fn prev_aux2_mode(&mut self) {
        self.aux2_mode = self.aux2_mode.prev();
    }

    pub fn setup_state(&self) -> SetupState {
        self.setup_state
    }

    pub fn next_setup_state(&mut self) {
        self.setup_state = match self.setup_state {
            SetupState::Idle => SetupState::Aux1,
            SetupState::Aux1 => SetupState::Aux2,
            SetupState::Aux2 => SetupState::Idle,
        };
    }
}

impl Setup {
    pub fn new(
        enc_a: impl Peripheral<P = impl Pin> + 'static,
        enc_b: impl Peripheral<P = impl Pin> + 'static,
        enc_sw: impl Peripheral<P = impl Pin> + 'static,
        led_din: impl Peripheral<P = impl Pin> + 'static,
    ) -> Self {
        let enc_a = Input::new(enc_a, Pull::Up);
        let enc_b = Input::new(enc_b, Pull::Up);
        let enc_sw = Input::new(enc_sw, Pull::Up);

        let last_a = enc_a.is_high();
        let last_b = enc_b.is_high();

        Self {
            enc_a,
            enc_b,
            enc_sw,

            led_din: Output::new(led_din, Level::Low, Speed::VeryHigh),

            last_a,
            last_b,
            sw_was_pressed: false,
        }
    }

    pub fn switch_pressed(&self) -> bool {
        self.enc_sw.is_low()
    }

    pub fn switch_just_pressed(&mut self) -> bool {
        let pressed = self.switch_pressed();
        let just_pressed = pressed && !self.sw_was_pressed;
        self.sw_was_pressed = pressed;
        just_pressed
    }

    pub fn poll_encoder(&mut self) -> Option<EncoderEvent> {
        let a = self.enc_a.is_high();
        let b = self.enc_b.is_high();

        let event = if self.last_a && !a {
            if b {
                Some(EncoderEvent::Clockwise)
            } else {
                Some(EncoderEvent::CounterClockwise)
            }
        } else {
            None
        };

        self.last_a = a;
        self.last_b = b;

        event
    }

    pub async fn show_state(
        &mut self,
        setup_state: SetupState,
        aux1_mode: AuxMode,
        aux2_mode: AuxMode,
        blink_on: bool,
    ) {
        let mut aux1_led = mode_color(aux1_mode);
        let mut aux2_led = mode_color(aux2_mode);

        match setup_state {
            SetupState::Idle => {}

            SetupState::Aux1 => {
                if !blink_on {
                    aux1_led = Rgb::BLACK;
                }
            }

            SetupState::Aux2 => {
                if !blink_on {
                    aux2_led = Rgb::BLACK;
                }
            }
        }

        self.write_ws2812([aux1_led, aux2_led]).await;
    }

    async fn write_ws2812(&mut self, leds: [Rgb; 2]) {
        cortex_m::interrupt::free(|_| {
            for led in leds {
                self.write_ws2812_byte(led.g);
                self.write_ws2812_byte(led.r);
                self.write_ws2812_byte(led.b);
            }

            self.led_din.set_low();
        });

        Timer::after_micros(WS2812_RESET_US).await;
    }

    fn write_ws2812_byte(&mut self, byte: u8) {
        for bit in (0..8).rev() {
            if (byte & (1 << bit)) != 0 {
                self.write_ws2812_one();
            } else {
                self.write_ws2812_zero();
            }
        }
    }

    #[inline(always)]
    fn write_ws2812_zero(&mut self) {
        self.led_din.set_high();
        cortex_m::asm::delay(WS2812_T0H_CYCLES);

        self.led_din.set_low();
        cortex_m::asm::delay(WS2812_T0L_CYCLES);
    }

    #[inline(always)]
    fn write_ws2812_one(&mut self) {
        self.led_din.set_high();
        cortex_m::asm::delay(WS2812_T1H_CYCLES);

        self.led_din.set_low();
        cortex_m::asm::delay(WS2812_T1L_CYCLES);
    }
}

impl<PIN> AuxPort<PIN>
where
    PIN: AdcChannel<ADC1>,
{
    pub fn new(
        sw1: impl Peripheral<P = impl Pin> + 'static,
        sw2: impl Peripheral<P = impl Pin> + 'static,
        pin_adc: PIN,
        state_pin: impl Peripheral<P = impl Pin> + 'static,
    ) -> Self {
        Self {
            sw1: Input::new(sw1, Pull::Up),
            sw2: Input::new(sw2, Pull::Up),
            adc: pin_adc,
            state_pin: Output::new(state_pin, Level::Low, Speed::Low),

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

    pub fn adc_read_raw(&mut self, adc: &mut Adc<'static, ADC1>) -> u16 {
        adc.blocking_read(&mut self.adc)
    }

    pub fn adc_read_f32(&mut self, adc: &mut Adc<'static, ADC1>) -> f32 {
        self.adc_read_raw(adc) as f32 / 65535.0
    }

    pub fn set_mode(&mut self, mode: AuxElectricalMode) {
        match mode {
            AuxElectricalMode::Footswitch => {
                self.state_pin.set_low();
            }
            AuxElectricalMode::Expression => {
                self.state_pin.set_high();
            }
        }
    }
}

impl<ADC_PIN1, ADC_PIN2> AUX<ADC_PIN1, ADC_PIN2>
where
    ADC_PIN1: AdcChannel<ADC1>,
    ADC_PIN2: AdcChannel<ADC1>,
{
    pub fn new(
        aux1_sw1: impl Peripheral<P = impl Pin> + 'static,
        aux1_sw2: impl Peripheral<P = impl Pin> + 'static,
        aux1_adc: ADC_PIN1,
        aux1_state_pin: impl Peripheral<P = impl Pin> + 'static,

        aux2_sw1: impl Peripheral<P = impl Pin> + 'static,
        aux2_sw2: impl Peripheral<P = impl Pin> + 'static,
        aux2_adc: ADC_PIN2,
        aux2_state_pin: impl Peripheral<P = impl Pin> + 'static,
    ) -> Self {
        Self {
            aux1: AuxPort::new(aux1_sw1, aux1_sw2, aux1_adc, aux1_state_pin),
            aux2: AuxPort::new(aux2_sw1, aux2_sw2, aux2_adc, aux2_state_pin),
        }
    }

    pub fn set_aux1_mode(&mut self, mode: AuxElectricalMode) {
        self.aux1.set_mode(mode);
    }

    pub fn set_aux2_mode(&mut self, mode: AuxElectricalMode) {
        self.aux2.set_mode(mode);
    }

    pub fn poll_aux1_event(&mut self) -> Option<SwitchEvent> {
        self.aux1.poll_event()
    }

    pub fn poll_aux2_event(&mut self) -> Option<SwitchEvent> {
        self.aux2.poll_event()
    }

    pub fn read_aux1_f32(&mut self, adc: &mut Adc<'static, ADC1>) -> f32 {
        self.aux1.adc_read_f32(adc)
    }

    pub fn read_aux2_f32(&mut self, adc: &mut Adc<'static, ADC1>) -> f32 {
        self.aux2.adc_read_f32(adc)
    }

    pub async fn run(mut self, mut adc: Adc<'static, ADC1>, mut leds: ChLeds) {
        let mut aux1_last_mode = AuxMode::Off;
        let mut aux2_last_mode = AuxMode::Off;

        let mut aux1_sw2_pressed_at: Option<Instant> = None;
        let mut aux2_sw2_pressed_at: Option<Instant> = None;

        let mut last_channel: Option<usize> = None;

        loop {
            let (aux1_mode, aux2_mode, channel) = {
                let state = STATE.lock().await;
                (state.aux1_mode(), state.aux2_mode(), state.channel())
            };

            if last_channel != Some(channel) {
                leds.set_channel(channel);
                last_channel = Some(channel);
            }

            if aux1_mode != aux1_last_mode {
                self.set_aux1_mode(aux1_mode.electrical_mode());
                aux1_last_mode = aux1_mode;
                aux1_sw2_pressed_at = None;
            }

            if aux2_mode != aux2_last_mode {
                self.set_aux2_mode(aux2_mode.electrical_mode());
                aux2_last_mode = aux2_mode;
                aux2_sw2_pressed_at = None;
            }

            if aux1_mode.is_footswitch() {
                if let Some(event) = self.poll_aux1_event() {
                    handle_aux_switch_event(1, aux1_mode, event, &mut aux1_sw2_pressed_at).await;
                }
            }

            if aux2_mode.is_footswitch() {
                if let Some(event) = self.poll_aux2_event() {
                    handle_aux_switch_event(2, aux2_mode, event, &mut aux2_sw2_pressed_at).await;
                }
            }

            if aux1_mode.is_expression() {
                let value = self.read_aux1_f32(&mut adc);
                handle_aux_expression(1, aux1_mode, value).await;
            }

            if aux2_mode.is_expression() {
                let value = self.read_aux2_f32(&mut adc);
                handle_aux_expression(2, aux2_mode, value).await;
            }

            Timer::after_millis(10).await;
        }
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

type AppAux = AUX<SeedPin15, SeedPin16>;

#[embassy_executor::task]
pub async fn aux_task(aux: AppAux, adc: Adc<'static, ADC1>, leds: ChLeds) {
    aux.run(adc, leds).await;
}

#[embassy_executor::task]
pub async fn setup_task(mut setup: Setup) {
    let mut last_setup_state = SetupState::Idle;
    let mut last_aux1_mode = AuxMode::Off;
    let mut last_aux2_mode = AuxMode::Off;

    let mut blink_on = true;
    let mut blink_counter = 0u32;

    setup
        .show_state(last_setup_state, last_aux1_mode, last_aux2_mode, blink_on)
        .await;

    loop {
        let mut redraw = false;

        if setup.switch_just_pressed() {
            let (setup_state, aux1_mode, aux2_mode) = {
                let mut state = STATE.lock().await;
                state.next_setup_state();

                (state.setup_state(), state.aux1_mode(), state.aux2_mode())
            };

            last_setup_state = setup_state;
            last_aux1_mode = aux1_mode;
            last_aux2_mode = aux2_mode;

            blink_on = true;
            blink_counter = 0;
            redraw = true;
        }

        if let Some(event) = setup.poll_encoder() {
            let (setup_state, aux1_mode, aux2_mode) = {
                let mut state = STATE.lock().await;

                match state.setup_state() {
                    SetupState::Idle => match event {
                        EncoderEvent::Clockwise => state.next_channel(),
                        EncoderEvent::CounterClockwise => state.prev_channel(),
                    },

                    SetupState::Aux1 => match event {
                        EncoderEvent::Clockwise => state.next_aux1_mode(),
                        EncoderEvent::CounterClockwise => state.prev_aux1_mode(),
                    },

                    SetupState::Aux2 => match event {
                        EncoderEvent::Clockwise => state.next_aux2_mode(),
                        EncoderEvent::CounterClockwise => state.prev_aux2_mode(),
                    },
                }

                (state.setup_state(), state.aux1_mode(), state.aux2_mode())
            };

            last_setup_state = setup_state;
            last_aux1_mode = aux1_mode;
            last_aux2_mode = aux2_mode;

            blink_on = true;
            blink_counter = 0;
            redraw = true;
        }

        if last_setup_state != SetupState::Idle {
            blink_counter += 1;

            if blink_counter >= 25 {
                blink_counter = 0;
                blink_on = !blink_on;
                redraw = true;
            }
        } else {
            if !blink_on {
                blink_on = true;
                redraw = true;
            }
        }

        let (setup_state, aux1_mode, aux2_mode) = {
            let state = STATE.lock().await;
            (state.setup_state(), state.aux1_mode(), state.aux2_mode())
        };

        if setup_state != last_setup_state
            || aux1_mode != last_aux1_mode
            || aux2_mode != last_aux2_mode
        {
            last_setup_state = setup_state;
            last_aux1_mode = aux1_mode;
            last_aux2_mode = aux2_mode;
            redraw = true;
        }

        if redraw {
            setup
                .show_state(last_setup_state, last_aux1_mode, last_aux2_mode, blink_on)
                .await;
        }

        Timer::after_millis(10).await;
    }
}
