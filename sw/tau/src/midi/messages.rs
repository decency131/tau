use crate::STATE;

use crate::config::MIDI_VELOCITY;

#[derive(Clone, Copy, PartialEq)]
pub enum MidiEvent {
    NoteOn { channel: u8, note: u8, velocity: u8 },
    NoteOff { channel: u8, note: u8 },
    ControlChange { channel: u8, cc: u8, value: u8 },
}

impl MidiEvent {
    pub fn to_usb_packet(self) -> [u8; 4] {
        match self {
            MidiEvent::NoteOn {
                channel,
                note,
                velocity,
            } => [0x09, 0x90 | (channel & 0x0f), note, velocity],

            MidiEvent::NoteOff { channel, note } => [0x08, 0x80 | (channel & 0x0f), note, 0],

            MidiEvent::ControlChange { channel, cc, value } => {
                [0x0B, 0xB0 | (channel & 0x0f), cc, value]
            }
        }
    }

    pub fn note_on(channel: usize, note: u8) -> Self {
        Self::NoteOn {
            channel: channel as u8,
            note,
            velocity: MIDI_VELOCITY,
        }
    }

    pub fn note_off(channel: usize, note: u8) -> Self {
        Self::NoteOff {
            channel: channel as u8,
            note,
        }
    }

    pub fn control_change(channel: usize, cc: u8, value: u8) -> Self {
        Self::ControlChange {
            channel: channel as u8,
            cc,
            value,
        }
    }
}
