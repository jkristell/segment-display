#![no_std]

//! A platform agnostic driver to interface with 7-segments displays
//! connected to shift registers
//! 
//! This is work-in-progress!
//!
//! Example
//!
//! ```no_run
//!
//!#![no_main]
//!#![no_std]
//!
//!extern crate cortex_m;
//!extern crate cortex_m_rt;
//!extern crate nucleo_f401re as board;
//!extern crate panic_semihosting;
//!
//!use cortex_m_rt::entry;
//!
//!use board::hal::delay::Delay;
//!use board::hal::prelude::*;
//!use board::hal::stm32;
//!use board::spi::{self, Spi};
//!use cortex_m::peripheral::Peripherals;
//!
//!use segment_display::SegmentDisplay;
//!
//!
//!#[entry]
//!fn main() -> ! {
//!    let device = stm32::Peripherals::take().unwrap();
//!    let core = Peripherals::take().unwrap();
//!
//!    let rcc = device.RCC.constrain();
//!    let clocks = rcc.cfgr.sysclk(84.mhz()).freeze();
//!
//!    let gpiob = device.GPIOB.split();
//!    let sck = gpiob.pb3.into_alternate_af5();
//!    let miso = spi::NoMiso;
//!    let mosi = gpiob.pb5.into_alternate_af5();
//!    let latch = gpiob.pb4.into_push_pull_output();
//!
//!    let spi = Spi::spi1(
//!        device.SPI1,
//!        (sck, miso, mosi),
//!        spi::Mode { polarity: spi::Polarity::IdleHigh, phase: spi::Phase::CaptureOnFirstTransition, },
//!        4_000_000.hz(),
//!        clocks,
//!    );
//!
//!    let mut segment_display = SegmentDisplay::new(spi, latch);
//!    let mut delay = Delay::new(core.SYST, clocks);
//!
//!    segment_display.write_str("HELO");
//!
//!    loop {
//!        segment_display.refresh().unwrap();
//!        delay.delay_us(1000_u16);
//!    }
//!}
//!
//! ```

use embedded_hal::blocking::delay::DelayUs;
use embedded_hal::blocking::spi;
use embedded_hal::digital::v2::OutputPin;

pub struct SegmentDisplay<SPI, PIN> {
    back_buffer: [u8; 4],
    spi: SPI,
    latch_pin: PIN,
    current_digit: usize,
}


#[derive(Debug)]
pub enum Error<SpiError, PinError> {
    Spi(SpiError),
    Pin(PinError)
}


impl<SPI, PIN> SegmentDisplay<SPI, PIN>
where
    SPI: spi::Write<u8>,
    PIN: OutputPin,
{
    /// Create a new SegmentDisplay
    pub fn new(spi: SPI, latch_pin: PIN) -> Self {
        Self {
            back_buffer: [0xff; 4],
            spi,
            latch_pin,
            current_digit: 0,
        }
    }

    /// Release the SegmentDisplay and the resources
    pub fn release(self) -> (SPI, PIN) {
        (self.spi, self.latch_pin)
    }

    /// Refresh the display. Needs to be called periodically with a sufficientlty hight frequenzy
    /// otherwise the display will flicker.
    pub fn refresh(&mut self) -> Result<(), Error<SPI::Error, PIN::Error>> {
        let segments_and_select: [u8; 2] = [
            // The segments in digit to turn on/off
            self.back_buffer[self.current_digit],
            // The current display selector.
            1 << (4 - 1 - self.current_digit),
        ];

        self.current_digit = (self.current_digit + 1) & 0b11;

        self.latch_pin.set_low().map_err(Error::Pin)?;

        let res = self.spi
                .write(&segments_and_select)
                .map_err(Error::Spi)?;

        self.latch_pin
                .set_high()
                .map_err(Error::Pin)?;

        Ok(res)
    }

    pub fn refresh_with_delay<DELAY>(&mut self, delay: &mut DELAY) -> Result<(), Error<SPI::Error, PIN::Error>>
    where
        DELAY: DelayUs<u16>,
    {
        let segments_and_select: [u8; 2] = [
            // The segments in digit to turn on/off
            self.back_buffer[self.current_digit],
            // The current display selector.
            1 << (4 - 1 - self.current_digit),
        ];

        self.current_digit = (self.current_digit + 1) & 0b11;

        self.latch_pin
                .set_low()
                .map_err(Error::Pin)?;
        let res = self.spi
                .write(&segments_and_select)
                .map_err(Error::Spi)?;

        delay.delay_us(100);
        self.latch_pin
                .set_high()
                .map_err(Error::Pin)?;

        Ok(res)
    }

    /// Write characters to the display
    pub fn write_chars(&mut self, buf: [char; 4]) {
        for (i, c) in buf.iter().enumerate() {
            self.back_buffer[i] = Self::char_to_segment_code(*c);
        }
    }

    /// Write a string to the display
    pub fn write_str(&mut self, s: &str) {

        self.back_buffer.iter_mut().for_each(|b| *b = !0);

        for (i, c) in s.chars().take(4).enumerate() {
            self.back_buffer[i] = Self::char_to_segment_code(c);
        }
    }

    /// Write a number to the display
    pub fn write_number(&mut self, num: usize) {
        let mut num = num;

        if num > 9999 {
            num = 9999;
        }

        for (i, div) in [1000, 100, 10].iter().enumerate() {
            let digit;
            if num >= i {
                digit = num / div;
                num -= div * digit;
            } else {
                digit = 0;
            }
            self.back_buffer[i] = NUMERALS[digit];
        }

        self.back_buffer[3] = NUMERALS[num];
    }

    fn char_to_segment_code(c: char) -> u8 {
        if c.is_ascii_digit() {
            let cb = c as u8;
            let idx = cb - ('0' as u8);
            NUMERALS[idx as usize]
        } else if c.is_ascii_alphabetic() {
            let cb = (c as u8) & !0x20; // Convert to uppercase
            let idx = cb - ('A' as u8);
            LETTERS[idx as usize]
        } else {
            // Symbols
            match c {
                ' ' => 0b1111_1111,
                '-' => 0b1011_1111,
                '_' => 0b1111_0111,
                _   => 0b1111_1111,
            }
        }
    }
}

//           A
//          ===
//      F ||   || B
//          =G=
//      E ||   || C
//          ===
//           D


static NUMERALS: [u8; 10] = [
    //.GFE_DCBA
    0b1100_0000,    // 0
    0b1111_1001,    // 1
    0b1010_0100,    // 2
    0b1011_0000,    // 3
    0b1001_1001,    // 4
    0b1001_0010,    // 5
    0b1000_0010,    // 6
    0b1111_1000,    // 7
    0b1000_0000,    // 8
    0b1001_1000,    // 9
];

static LETTERS: [u8; 26] = [
    0b1000_1000,    // A
    0b1000_0011,    // B
    0b1100_0110,    // C
    0b1010_0001,    // D
    0b1000_0110,    // E
    0b1000_1110,    // F
    0b1100_0010,    // G
    0b1000_1001,    // H
    0b1100_1111,    // I
    0b1110_0001,    // J
    0b1000_1010,    // K
    0b1100_0111,    // L
    0b1110_1010,    // M
    0b1100_1000,    // N
    0b1100_0000,    // O
    0b1000_1100,    // P
    0b1001_0100,    // Q
    0b1100_1100,    // R
    0b1001_0010,    // S
    0b1000_0111,    // T
    0b1100_0001,    // U
    0b1100_0001,    // V
    0b1101_0101,    // W
    0b1000_1001,    // X
    0b1001_0001,    // Y
    0b1010_0100,    // Z
];
