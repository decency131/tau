use defmt::{info, warn};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Instant, Timer};
use heapless::Vec;

use crate::config::MAX_EVENTS;
use crate::midi::MidiEvent;
use crate::midi::usb::{MIDI_IN_CH, send_usb_event};

pub static LOOPER_CONTROL_CH: Channel<CriticalSectionRawMutex, LooperControl, 4> = Channel::new();

#[derive(Clone, Copy)]
pub enum LooperControl {
    Sw1,
    PauseResume,
    Clear,
}

#[derive(Clone, Copy, PartialEq)]
pub enum LooperState {
    Idle,
    Recording,
    Playing,
    Overdub,
    Paused,
}

#[derive(Clone, Copy)]
struct MidiLooperEvent {
    time_ms: u32,
    event: MidiEvent,
}

pub struct Looper {
    events: Vec<MidiLooperEvent, MAX_EVENTS>,
    state: LooperState,

    record_start_ms: u32,
    loop_len_ms: u32,

    playback_start_ms: u32,
    last_pos_ms: u32,
    paused_pos_ms: u32,
}

impl Looper {
    pub const fn new() -> Self {
        Self {
            events: Vec::new(),
            state: LooperState::Idle,

            record_start_ms: 0,
            loop_len_ms: 0,

            playback_start_ms: 0,
            last_pos_ms: 0,
            paused_pos_ms: 0,
        }
    }

    fn now_ms() -> u32 {
        Instant::now().as_millis() as u32
    }

    fn sw1(&mut self) {
        let now = Self::now_ms();

        match self.state {
            LooperState::Idle => {
                self.events.clear();
                self.record_start_ms = now;
                self.loop_len_ms = 0;
                self.state = LooperState::Recording;
                info!("looper: recording");
            }

            LooperState::Recording => {
                self.loop_len_ms = now.wrapping_sub(self.record_start_ms).max(1);
                self.playback_start_ms = now;
                self.last_pos_ms = 0;
                self.state = LooperState::Playing;
                info!("looper: playing len_ms={}", self.loop_len_ms);
            }

            LooperState::Playing => {
                self.state = LooperState::Overdub;
                info!("looper: overdub");
            }

            LooperState::Overdub => {
                self.state = LooperState::Playing;
                info!("looper: playing");
            }

            LooperState::Paused => {
                self.playback_start_ms = now.wrapping_sub(self.paused_pos_ms);
                self.last_pos_ms = self.paused_pos_ms;
                self.state = LooperState::Playing;
                info!("looper: resume");
            }
        }
    }

    fn pause_resume(&mut self) {
        let now = Self::now_ms();

        match self.state {
            LooperState::Playing | LooperState::Overdub => {
                self.paused_pos_ms = self.current_pos_ms(now);
                self.send_all_notes_off();
                self.state = LooperState::Paused;
                info!("looper: paused");
            }

            LooperState::Paused => {
                self.playback_start_ms = now.wrapping_sub(self.paused_pos_ms);
                self.last_pos_ms = self.paused_pos_ms;
                self.state = LooperState::Playing;
                info!("looper: resumed");
            }

            _ => {}
        }
    }

    fn clear(&mut self) {
        self.events.clear();
        self.loop_len_ms = 0;
        self.last_pos_ms = 0;
        self.paused_pos_ms = 0;
        self.send_all_notes_off();
        self.state = LooperState::Idle;
        info!("looper: cleared");
    }

    fn handle_input_event(&mut self, event: MidiEvent) {
        let now = Self::now_ms();

        match self.state {
            LooperState::Idle => {
                send_usb_event(event);
            }

            LooperState::Recording => {
                let time_ms = now.wrapping_sub(self.record_start_ms);
                self.record_event(time_ms, event);
                send_usb_event(event);
            }

            LooperState::Playing | LooperState::Paused => {
                // Dry/live MIDI still passes through.
                send_usb_event(event);
            }

            LooperState::Overdub => {
                let time_ms = self.current_pos_ms(now);
                self.record_event(time_ms, event);
                send_usb_event(event);
            }
        }
    }

    fn record_event(&mut self, time_ms: u32, event: MidiEvent) {
        if self
            .events
            .push(MidiLooperEvent { time_ms, event })
            .is_err()
        {
            warn!("looper event buffer full");
        }
    }

    fn current_pos_ms(&self, now: u32) -> u32 {
        if self.loop_len_ms == 0 {
            0
        } else {
            now.wrapping_sub(self.playback_start_ms) % self.loop_len_ms
        }
    }

    fn tick(&mut self) {
        match self.state {
            LooperState::Playing | LooperState::Overdub => {}
            _ => return,
        }

        if self.loop_len_ms == 0 {
            return;
        }

        let now = Self::now_ms();
        let pos = self.current_pos_ms(now);
        let last = self.last_pos_ms;

        if pos == last {
            return;
        }

        let wrapped = pos < last;

        for e in self.events.iter() {
            let should_play = if wrapped {
                e.time_ms > last || e.time_ms <= pos
            } else {
                e.time_ms > last && e.time_ms <= pos
            };

            if should_play {
                send_usb_event(e.event);
            }
        }

        self.last_pos_ms = pos;
    }

    fn send_all_notes_off(&self) {
        send_usb_event(MidiEvent::ControlChange(123, 0));
    }
}

pub fn queue_looper_control(control: LooperControl) {
    if LOOPER_CONTROL_CH.try_send(control).is_err() {
        warn!("looper control queue full");
    }
}

#[embassy_executor::task]
pub async fn looper_task() {
    let mut looper = Looper::new();

    loop {
        while let Ok(control) = LOOPER_CONTROL_CH.try_receive() {
            match control {
                LooperControl::Sw1 => looper.sw1(),
                LooperControl::PauseResume => looper.pause_resume(),
                LooperControl::Clear => looper.clear(),
            }
        }

        while let Ok(event) = MIDI_IN_CH.try_receive() {
            looper.handle_input_event(event);
        }

        looper.tick();

        Timer::after_millis(1).await;
    }
}
