use crate::STATE;

use crate::config::MIDI_VELOCITY;

#[derive(Clone, Copy, PartialEq)]
pub enum MidiEvent {
    NoteOn(u8),
    NoteOff(u8),
    ControlChange(u8, u8),
}

impl MidiEvent {
    pub async fn to_usb_packet(self) -> [u8; 4] {
        let channel = {
            let mut state = STATE.lock().await;
            state.channel()
        };

        let ch = channel as u8 & 0x0f;

        match self {
            MidiEvent::NoteOn(note) => [0x09, 0x90 | ch, note, MIDI_VELOCITY],

            MidiEvent::NoteOff(note) => [0x08, 0x80 | ch, note, 0],

            MidiEvent::ControlChange(cc, value) => [0x0B, 0xB0 | ch, cc, value],
        }
    }
}
