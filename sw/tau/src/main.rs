#![no_std]
#![no_main]

mod audio;
mod board;
mod config;
mod dsp;
mod midi;
mod utils;

use cortex_m::peripheral::Peripherals;
use daisy_embassy::{
    hal::{self, bind_interrupts, peripherals, usb},
    led::UserLed,
    new_daisy_board,
};
use defmt::{info, unwrap};
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_stm32::interrupt::{InterruptExt, Priority};
use embassy_stm32::usb::{Config, Driver};
use embassy_stm32::{gpio::*, interrupt};
use embassy_time::Timer;
use embassy_usb::Builder;
use embassy_usb::class::midi::MidiClass;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use crate::audio::input::{AUDIO_EXECUTOR, audio_task};
use crate::config::{USB_MANUFACTURER, USB_PID, USB_PRODUCT, USB_SERIAL, USB_VID};
use crate::dsp::pitch::pitch_task;
use crate::midi::usb::usb_midi_task;

bind_interrupts!(pub struct UsbIrqs {
    OTG_HS => usb::InterruptHandler<peripherals::USB_OTG_HS>;
});

#[embassy_stm32::interrupt]
unsafe fn SAI1() {
    unsafe { AUDIO_EXECUTOR.on_interrupt() }
}

#[embassy_executor::task]
async fn blink(mut led: UserLed<'static>) {
    loop {
        led.on();
        Timer::after_millis(500).await;

        led.off();
        Timer::after_millis(500).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("==== pitch detector start ====");

    let mut cp = Peripherals::take().unwrap();
    let rcc_config = daisy_embassy::default_rcc();

    cp.SCB.enable_fpu();
    cp.SCB.enable_icache();

    let p = hal::init(rcc_config);
    let board = new_daisy_board!(p);

    let ch_leds = board::ChLeds::new(
        board.pins.d13,
        board.pins.d12,
        board.pins.d11,
        board.pins.d10,
    );

    let midi_enable = board::MIDIEnable::new(board.pins.d14);

    spawner.spawn(blink(board.user_led)).unwrap();
    spawner.spawn(pitch_task(midi_enable)).unwrap();

    let interface = board
        .audio_peripherals
        .prepare_interface(Default::default())
        .await;

    interrupt::SAI1.set_priority(Priority::P6);
    let audio_spawner = AUDIO_EXECUTOR.start(interrupt::SAI1);
    unwrap!(audio_spawner.spawn(audio_task(interface)));

    let mut usb_config = Config::default();
    usb_config.vbus_detection = false;

    static EP_OUT_BUFFER: StaticCell<[u8; 256]> = StaticCell::new();
    let ep_out_buffer = EP_OUT_BUFFER.init([0; 256]);

    let driver = Driver::new_fs(
        p.USB_OTG_HS,
        UsbIrqs,
        board.pins.d30,
        board.pins.d29,
        ep_out_buffer,
        usb_config,
    );

    let mut device_config = embassy_usb::Config::new(USB_VID, USB_PID);
    device_config.manufacturer = Some(USB_MANUFACTURER);
    device_config.product = Some(USB_PRODUCT);
    device_config.serial_number = Some(USB_SERIAL);
    device_config.device_class = 0xEF;
    device_config.device_sub_class = 0x02;
    device_config.device_protocol = 0x01;
    device_config.composite_with_iads = true;

    static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

    let config_descriptor = CONFIG_DESCRIPTOR.init([0; 256]);
    let bos_descriptor = BOS_DESCRIPTOR.init([0; 256]);
    let control_buf = CONTROL_BUF.init([0; 64]);

    let mut builder = Builder::new(
        driver,
        device_config,
        config_descriptor,
        bos_descriptor,
        &mut [],
        control_buf,
    );

    let mut midi_class = MidiClass::new(&mut builder, 1, 1, 64);
    //info!("before usb builder");
    let mut usb = builder.build();

    info!("USB MIDI ready; waiting for host");
    //info!("before usb run");
    join(usb.run(), usb_midi_task(&mut midi_class)).await;
}
