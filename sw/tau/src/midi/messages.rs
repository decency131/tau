use crate::STATE;

use crate::config::MIDI_VELOCITY;

#[derive(Clone, Copy)]
pub enum MidiEvent {
    NoteOn(u8),
    NoteOff(u8),
}

impl MidiEvent {
    pub async fn to_usb_packet(self) -> [u8; 4] {
        let channel = {
            let mut state = STATE.lock().await;
            state.channel()
        };
        match self {
            MidiEvent::NoteOn(note) => [0x09, 0x90 | (channel as u8 & 0x0f), note, MIDI_VELOCITY],
            MidiEvent::NoteOff(note) => [0x08, 0x80 | (channel as u8 & 0x0f), note, 0],
        }
    }
}
