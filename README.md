# segment-display

[![docs.rs](https://docs.rs/segment-display/badge.svg)](https://docs.rs/segment-display/)

This is a driver crate for simple seven segments displays.

This crate was created for a seven-segment module made by 
["Qifei" marked with the serial number "3641BS"](https://www.electrokit.com/produkt/seriell-display-7-segment-4-siffror-74hc595/). 
The PCB  is fitted with two 74HC595 shift registers that are then connected to the 4 seven-segment displays.

The nice thing about these pcbs are that they only require 3 pins clock, 
data and latch to control. Another nice thing is that you can use the Spi-controller 
for doing this communication. The downside is that the digits are all multiplexed and 
therefore you have to cycle through them, update every digit in turn fast enough to fool the eye.

Right now this crate is tailored for this particular PCB and setup, but it shouldn't be too hard to modify it to support other configurations.

A simple example on how to use this driver is included below, and two other examples are available in my [nucleo-f401re](https://github.com/jkristell/nucleo-f401re/tree/segment-display/examples) crate.
The RTFM based one is particularly nice as it does the refresh in a RTFM scheduled task.


 ```rust
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt;
extern crate nucleo_f401re as board;
extern crate panic_semihosting;

use cortex_m_rt::entry;

use board::hal::delay::Delay;
use board::hal::prelude::*;
use board::hal::stm32;
use board::spi::{self, Spi};
use cortex_m::peripheral::Peripherals;

use segment_display::SegmentDisplay;


#[entry]
fn main() -> ! {
    let device = stm32::Peripherals::take().unwrap();
    let core = Peripherals::take().unwrap();

    let rcc = device.RCC.constrain();
    let clocks = rcc.cfgr.sysclk(84.mhz()).freeze();

    let gpiob = device.GPIOB.split();
    let sck = gpiob.pb3.into_alternate_af5();
    let miso = spi::NoMiso;
    let mosi = gpiob.pb5.into_alternate_af5();
    let latch = gpiob.pb4.into_push_pull_output();

    let spi = Spi::spi1(
        device.SPI1,
        (sck, miso, mosi),
        spi::Mode { polarity: spi::Polarity::IdleHigh, phase: spi::Phase::CaptureOnFirstTransition, },
        4_000_000.hz(),
        clocks,
    );

    let mut segment_display = SegmentDisplay::new(spi, latch);
    let mut delay = Delay::new(core.SYST, clocks);

    segment_display.write_str("HELO");

    loop {
        segment_display.refresh().unwrap();
        delay.delay_us(1000_u16);
    }
}

 ```
