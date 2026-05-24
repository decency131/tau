use core::num::Wrapping;

use crate::config::{MIDI_MAX_NOTE, MIDI_MIN_NOTE};

#[inline(always)]
pub fn u24_to_f32(y: u32) -> f32 {
    let y = (Wrapping(y) + Wrapping(0x0080_0000)).0 & 0x00FF_FFFF;
    (y as f32 / 8_388_608.0) - 1.0
}

pub fn pitch_hz_to_midi_note(hz: f32) -> Option<u8> {
    if hz <= 0.0 || !hz.is_finite() {
        return None;
    }

    let note_f = 69.0 + 12.0 * libm::log2f(hz / 440.0);
    let note_i = (note_f + 0.5) as i32;

    if note_i < MIDI_MIN_NOTE as i32 || note_i > MIDI_MAX_NOTE as i32 {
        None
    } else {
        Some(note_i as u8)
    }
}
