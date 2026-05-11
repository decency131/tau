use defmt::{info, panic};
use embassy_stm32::usb::{Driver, Instance};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_usb::class::midi::MidiClass;
use embassy_usb::driver::EndpointError;

use crate::midi::MidiEvent;

pub static MIDI_IN_CH: Channel<CriticalSectionRawMutex, MidiEvent, 16> = Channel::new();
pub static MIDI_OUT_CH: Channel<CriticalSectionRawMutex, MidiEvent, 16> = Channel::new();

pub fn queue_midi_event(event: MidiEvent) {
    if MIDI_IN_CH.try_send(event).is_err() {
        defmt::warn!("midi input queue full; dropping event");
    }
}

pub fn send_usb_event(event: MidiEvent) {
    if MIDI_OUT_CH.try_send(event).is_err() {
        defmt::warn!("midi usb queue full; dropping event");
    }
}

pub fn send_note_on(note: u8) {
    queue_midi_event(MidiEvent::NoteOn(note));
}

pub fn send_note_off(note: u8) {
    queue_midi_event(MidiEvent::NoteOff(note));
}

pub struct Disconnected {}

impl From<EndpointError> for Disconnected {
    fn from(val: EndpointError) -> Self {
        match val {
            EndpointError::BufferOverflow => panic!("USB MIDI buffer overflow"),
            EndpointError::Disabled => Disconnected {},
        }
    }
}

async fn midi_event_loop<'d, T: Instance + 'd>(
    class: &mut MidiClass<'d, Driver<'d, T>>,
) -> Result<(), Disconnected> {
    loop {
        let event = MIDI_OUT_CH.receive().await;
        let packet = event.to_usb_packet().await;
        class.write_packet(&packet).await?;
    }
}

pub async fn usb_midi_task<'d, T: Instance + 'd>(class: &mut MidiClass<'d, Driver<'d, T>>) {
    loop {
        class.wait_connection().await;
        info!("USB MIDI connected");

        let _ = midi_event_loop(class).await;

        info!("USB MIDI disconnected");
    }
}
