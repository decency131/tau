// Audio / DSP
pub const SAMPLE_RATE: f32 = 48_000.0;
pub const FRAME_LEN: usize = 512;

pub const HOP_LEN: usize = 256;

// YIN
pub const THRESHOLD: f32 = 0.20;
pub const MIN_PROBABILITY: f32 = 0.05;
pub const TAU_MAX: usize = 400;

// Throttling
pub const DETECT_EVERY_N_FRAMES: u32 = 2;
pub const PRINT_EVERY_N_FRAMES: u32 = 20;
pub const SILENCE_PRINT_EVERY_N_FRAMES: u32 = 32;

// Input channel selection
pub const INPUT_STRIDE: usize = 2;
pub const INPUT_OFFSET: usize = 0;

// Noise gate
pub const MIN_RMS: f32 = 0.004;
pub const MIN_RMS_SQUARED: f32 = MIN_RMS * MIN_RMS;
pub const MIN_PEAK: f32 = 0.012;

// MIDI
pub const MIDI_VELOCITY: u8 = 100;
pub const MIDI_MIN_NOTE: u8 = 24;
pub const MIDI_MAX_NOTE: u8 = 96;
pub const NOTE_OFF_AFTER_MISSES: u8 = 2;

// USB identity
pub const USB_VID: u16 = 0xdead;
pub const USB_PID: u16 = 0xc0de;
pub const USB_MANUFACTURER: &str = "decency131";
pub const USB_PRODUCT: &str = "tau";
pub const USB_SERIAL: &str = "12345678";
