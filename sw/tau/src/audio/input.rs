use daisy_embassy::audio::{Idle, Interface};
use defmt::{info, unwrap, warn};
use embassy_executor::InterruptExecutor;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};

use crate::config::{
    DETECT_EVERY_N_FRAMES, FRAME_LEN, INPUT_OFFSET, INPUT_STRIDE, MIN_PEAK, MIN_RMS,
    PRINT_EVERY_N_FRAMES, TAU_MAX,
};

pub static FRAME_CH: Channel<CriticalSectionRawMutex, [u32; FRAME_LEN], 2> = Channel::new();

pub static AUDIO_EXECUTOR: InterruptExecutor = InterruptExecutor::new();

#[embassy_executor::task]
pub async fn audio_task(interface: Interface<'static, Idle>) {
    let silence = 0u32;
    let mut raw_frame = [0u32; FRAME_LEN];
    let mut frame_i = 0usize;

    let input_stride = if INPUT_STRIDE == 0 {
        warn!("invalid INPUT_STRIDE=0; using 1");
        1
    } else {
        INPUT_STRIDE
    };

    let input_offset = if INPUT_OFFSET < input_stride {
        INPUT_OFFSET
    } else {
        warn!("invalid INPUT_OFFSET for INPUT_STRIDE; using 0");
        0
    };

    info!(
        "audio input config: stride={} offset={} frame_len={} tau_max={} detect_every={} print_every={} min_rms={} min_peak={}",
        input_stride,
        input_offset,
        FRAME_LEN,
        TAU_MAX,
        DETECT_EVERY_N_FRAMES,
        PRINT_EVERY_N_FRAMES,
        MIN_RMS,
        MIN_PEAK
    );

    let mut sample_phase = 0usize;
    let mut dropped_frames = 0u32;
    let mut interface = unwrap!(interface.start_interface().await);

    loop {
        let callback_result = interface
            .start_callback(|input, output| {
                output.fill(silence);

                for &sample in input.iter() {
                    if sample_phase == input_offset {
                        raw_frame[frame_i] = sample;
                        frame_i += 1;

                        if frame_i == FRAME_LEN {
                            frame_i = 0;

                            match FRAME_CH.try_send(raw_frame) {
                                Ok(()) => {}
                                Err(_) => {
                                    dropped_frames = dropped_frames.wrapping_add(1);
                                    if dropped_frames % 128 == 1 {
                                        warn!("pitch analyzer behind; dropping frames");
                                    }
                                }
                            }

                            raw_frame = [0u32; FRAME_LEN];
                        }
                    }

                    sample_phase += 1;
                    if sample_phase >= input_stride {
                        sample_phase = 0;
                    }
                }
            })
            .await;

        if callback_result.is_err() {
            warn!("audio callback stopped: overrun/error; restarting callback loop");
        }
    }
}
