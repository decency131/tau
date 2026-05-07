use crate::config::{MIDI_CHANNEL, MIDI_VELOCITY};

#[derive(Clone, Copy)]
pub enum MidiEvent {
    NoteOn(u8),
    NoteOff(u8),
}

impl MidiEvent {
    pub fn to_usb_packet(self) -> [u8; 4] {
        match self {
            MidiEvent::NoteOn(note) => [0x09, 0x90 | (MIDI_CHANNEL & 0x0f), note, MIDI_VELOCITY],
            MidiEvent::NoteOff(note) => [0x08, 0x80 | (MIDI_CHANNEL & 0x0f), note, 0],
        }
    }
}
