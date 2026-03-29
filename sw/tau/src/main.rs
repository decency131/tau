#![no_std]
#![no_main]

use cortex_m::peripheral::Peripherals;
use daisy_embassy::{DaisyBoard, hal, new_daisy_board};
use defmt::info;
use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Timer};
use {defmt_rtt as _, panic_probe as _};

use yin_no_std::Yin;

const SAMPLE_RATE: f32 = 48_000.0;
const THRESHOLD: f32 = 0.10;
const MIN_PROBABILITY: f32 = 0.15;

const FRAME_LEN: usize = 1024;
const TAU_MAX: usize = 512;
const ITERATIONS: usize = 1000;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("starting yin bench");

    let mut cp = Peripherals::take().unwrap();
    let config = daisy_embassy::default_rcc();

    cp.SCB.enable_fpu();
    cp.SCB.enable_icache();
    cp.SCB.enable_dcache(&mut cp.CPUID);

    let p = hal::init(config);
    let _board: DaisyBoard<'_> = new_daisy_board!(p);

    let yin = Yin::new(SAMPLE_RATE, THRESHOLD, MIN_PROBABILITY);

    let mut frame = [0.0f32; FRAME_LEN];
    make_sine(&mut frame, SAMPLE_RATE, 220.0);

    let mut diff = [0.0f32; TAU_MAX + 1];
    let mut cmnd = [0.0f32; TAU_MAX + 1];

    // warmup
    for _ in 0..100 {
        let _ = yin.detect(&frame, TAU_MAX, &mut diff, &mut cmnd);
    }

    let start = Instant::now();

    let mut found = 0usize;
    let mut last_hz = 0.0f32;
    let mut last_prob = 0.0f32;

    for _ in 0..ITERATIONS {
        if let Some(pitch) = yin.detect(&frame, TAU_MAX, &mut diff, &mut cmnd) {
            found += 1;
            last_hz = pitch.frequency_hz;
            last_prob = pitch.probability;
        }
    }

    let elapsed = start.elapsed();
    let total_us = elapsed.as_micros();
    let avg_us = total_us / (ITERATIONS as u64);

    info!("iterations = {}", ITERATIONS);
    info!("total_us = {}", total_us);
    info!("avg_us = {}", avg_us);
    info!("detections = {}", found);
    info!("last_hz = {}", last_hz);
    info!("last_probability = {}", last_prob);

    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}

fn make_sine(buf: &mut [f32], sample_rate: f32, freq_hz: f32) {
    let phase_inc = 2.0 * core::f32::consts::PI * freq_hz / sample_rate;
    let mut phase = 0.0f32;

    for x in buf.iter_mut() {
        *x = libm::sinf(phase);
        phase += phase_inc;
    }
}
