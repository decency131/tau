use crate::board::MIDIEnable;
use crate::dsp::smoothing::NoteSmoother;

use defmt::{debug, info};
use yin_no_std::Yin;

use crate::audio::input::FRAME_CH;
use crate::config::{
    DETECT_EVERY_N_FRAMES, FRAME_LEN, MIN_PEAK, MIN_PROBABILITY, MIN_RMS_SQUARED,
    NOTE_OFF_AFTER_MISSES, PRINT_EVERY_N_FRAMES, SAMPLE_RATE, SILENCE_PRINT_EVERY_N_FRAMES,
    TAU_MAX, THRESHOLD,
};
use crate::midi::usb::{send_note_off, send_note_on};
use crate::utils::{pitch_hz_to_midi_note, u24_to_f32};

#[derive(Clone, Copy)]
struct ActiveNote {
    note: u8,
    channel: usize,
}

#[embassy_executor::task]
pub async fn pitch_task(midi_enable: MIDIEnable) {
    let yin = Yin::new(SAMPLE_RATE, THRESHOLD, MIN_PROBABILITY);
    let mut note_smoother = NoteSmoother::new();

    let mut frame = [0.0f32; FRAME_LEN];
    let mut diff = [0.0f32; TAU_MAX + 1];
    let mut cmnd = [0.0f32; TAU_MAX + 1];

    let mut analyzed_frames = 0u32;
    let mut last_pitch_hz = 0.0f32;
    let mut last_probability = 0.0f32;
    let mut active_note: Option<ActiveNote> = None;
    let mut missed_detections = 0u8;

    let mut midi_switch_was_enabled = midi_enable.is_enabled();

    loop {
        let current_channel = {
            let state = crate::STATE.lock().await;
            state.channel()
        };

        let midi_enabled = midi_enable.is_enabled();

        if midi_switch_was_enabled && !midi_enabled {
            if let Some(active) = active_note.take() {
                send_note_off(active.channel, active.note);
                info!(
                    "midi note_off={} channel={} reason=midi_disabled",
                    active.note,
                    active.channel + 1
                );
            }
            missed_detections = 0;
        } else if !midi_switch_was_enabled && midi_enabled {
            info!("midi enabled");
        }

        midi_switch_was_enabled = midi_enabled;

        let raw_frame = FRAME_CH.receive().await;
        analyzed_frames = analyzed_frames.wrapping_add(1);

        let mut mean = 0.0f32;
        let mut raw_min = u32::MAX;
        let mut raw_max = 0u32;

        for i in 0..FRAME_LEN {
            let raw = raw_frame[i];

            if raw < raw_min {
                raw_min = raw;
            }

            if raw > raw_max {
                raw_max = raw;
            }

            let x = u24_to_f32(raw);
            frame[i] = x;
            mean += x;
        }

        mean /= FRAME_LEN as f32;

        let mut energy = 0.0f32;
        let mut peak = 0.0f32;

        for i in 0..FRAME_LEN {
            let x = frame[i] - mean;
            frame[i] = x;

            let ax = x.abs();
            energy += x * x;

            if ax > peak {
                peak = ax;
            }
        }

        let mean_square = energy / FRAME_LEN as f32;
        let rms = libm::sqrtf(mean_square);

        if analyzed_frames % 32 == 1 {
            debug!(
                "input_debug frame={} raw_min={} raw_max={} mean={} rms={} peak={} midi_en={}",
                analyzed_frames, raw_min, raw_max, mean, rms, peak, midi_enabled
            );
        }

        if mean_square < MIN_RMS_SQUARED || peak < MIN_PEAK {
            note_smoother.reset();

            last_pitch_hz = 0.0;
            last_probability = 0.0;
            missed_detections = 0;

            if let Some(active) = active_note.take() {
                send_note_off(active.channel, active.note);
                info!(
                    "midi note_off={} channel={} reason=silence",
                    active.note,
                    active.channel + 1
                );
            }

            if analyzed_frames % SILENCE_PRINT_EVERY_N_FRAMES == 1 {
                info!("pitch_hz=0 probability=0 rms={} peak={}", rms, peak);
            }

            continue;
        }

        if (analyzed_frames % DETECT_EVERY_N_FRAMES == 0)
        /* && midi_enabled*/
        {
            let detected_note = match yin.detect(&frame, TAU_MAX, &mut diff, &mut cmnd) {
                Some(pitch) => {
                    last_pitch_hz = pitch.frequency_hz;
                    last_probability = pitch.probability;

                    let raw_note = pitch_hz_to_midi_note(pitch.frequency_hz);
                    note_smoother.update(raw_note)
                }

                None => {
                    //note_smoother.reset();

                    last_pitch_hz = 0.0;
                    last_probability = 0.0;

                    None
                }
            };

            match detected_note {
                Some(note) => {
                    missed_detections = 0;

                    let needs_new_note = match active_note {
                        Some(active) => active.note != note || active.channel != current_channel,
                        None => true,
                    };

                    if needs_new_note {
                        if let Some(old) = active_note.take() {
                            send_note_off(old.channel, old.note);
                            info!(
                                "midi note_off={} channel={} reason=changed",
                                old.note,
                                old.channel + 1
                            );
                        }

                        send_note_on(current_channel, note);
                        info!(
                            "midi note_on={} channel={} pitch_hz={} probability={}",
                            note,
                            current_channel + 1,
                            last_pitch_hz,
                            last_probability
                        );

                        active_note = Some(ActiveNote {
                            note,
                            channel: current_channel,
                        });
                    }
                }
                None => {
                    if active_note.is_some() {
                        missed_detections = missed_detections.saturating_add(1);

                        if missed_detections >= NOTE_OFF_AFTER_MISSES {
                            if let Some(active) = active_note.take() {
                                send_note_off(active.channel, active.note);
                                info!(
                                    "midi note_off={} channel={} reason=lost_pitch",
                                    active.note,
                                    active.channel + 1
                                );
                            }

                            missed_detections = 0;
                        }
                    }
                    note_smoother.reset();
                }
            }
        }

        if analyzed_frames % PRINT_EVERY_N_FRAMES == 0 {
            info!(
                "pitch_hz={} probability={} rms={} peak={}",
                last_pitch_hz, last_probability, rms, peak
            );
        }
    }
}
