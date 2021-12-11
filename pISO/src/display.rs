use bitmap::Bitmap;
use spidev::{SPI_MODE_0, Spidev, SpidevOptions};
use std::io::Write;
use std::thread;
use std::time;
use sysfs_gpio::{Direction, Pin};
use error;
use config;

pub const DISPLAY_WIDTH: usize = 128;
pub const DISPLAY_HEIGHT: usize = 64;

#[allow(unused)]
enum SH1106Command {
    SetContrast = 0x81,
    DisplayAllOnResume = 0xA4,
    DisplayAllOn = 0xA5,
    NormalDisplay = 0xA6,
    InvertDisplay = 0xA7,
    DisplayOff = 0xAE,
    DisplayOn = 0xAF,
    SetDisplayOffset = 0xD3,
    SetComPins = 0xDA,
    SetVComDetect = 0xDB,
    SetDisplayClockDiv = 0xD5,
    SetPrecharge = 0xD9,
    SetMultiplex = 0xA8,
    SetPage = 0xB0,
    SetLowColumn = 0x00,
    SetHighColumn = 0x10,
    SetStartLine = 0x40,
    MemoryMode = 0x20,
    ComScanInc = 0xC0,
    ComScanDec = 0xC8,
    SegRemap = 0xA0,
    ChargePump = 0x8D,
    ExternalVCC = 0x1,
    SwitchcapVcc = 0x2,
}

impl Into<u8> for SH1106Command {
    fn into(self) -> u8 {
        self as u8
    }
}

pub struct LedDisplay {
    inverted: bool,
    contents: Bitmap,
    width: usize,
    height: usize,
    dc_pin: Pin,
    rst_pin: Pin,
    bus: Spidev,
}

pub trait Display {
    fn on(&mut self) -> error::Result<()>;
    fn reset(&mut self) -> error::Result<()>;
    fn update(&mut self, bitmap: Bitmap) -> error::Result<()>;
    fn flip_display(&mut self);
    fn width(&self) -> usize;
    fn height(&self) -> usize;
}

impl LedDisplay {
    pub fn new(config: &config::Config) -> error::Result<Box<Display>> {
        let mut spi = Spidev::open("/dev/spidev0.0")?;
        let options = SpidevOptions::new()
            .bits_per_word(8)
            .max_speed_hz(8000000)
            .mode(SPI_MODE_0)
            .build();
        spi.configure(&options)?;

        let dc_pin = Pin::new(config.pins.dc_pin);
        dc_pin.export()?;
        dc_pin.set_direction(Direction::Out)?;

        let rst_pin = Pin::new(config.pins.rst_pin);
        rst_pin.export()?;
        rst_pin.set_direction(Direction::Out)?;

        Ok(Box::new(LedDisplay {
            inverted: true,
            width: DISPLAY_WIDTH,
            height: DISPLAY_HEIGHT,
            contents: Bitmap::new(DISPLAY_WIDTH, DISPLAY_HEIGHT),
            dc_pin: dc_pin,
            rst_pin: rst_pin,
            bus: spi,
        }))
    }

    fn send_spi_command<Cmd>(&mut self, cmd: Cmd) -> error::Result<()>
    where
        Cmd: Into<u8>,
    {
        self.dc_pin.set_value(0)?;
        self.bus.write(&[cmd.into()])?;
        Ok(())
    }

    fn send_spi_data(&mut self, data: &[u8]) -> error::Result<()> {
        self.dc_pin.set_value(1)?;
        self.bus.write(data)?;
        Ok(())
    }
}

impl Display for LedDisplay {
    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        self.height
    }

    fn on(&mut self) -> error::Result<()> {
        self.reset()?;

        self.send_spi_command(SH1106Command::DisplayOff)?;
        self.send_spi_command(SH1106Command::SetDisplayClockDiv)?;
        self.send_spi_command(0x80)?; // the suggested ratio 0x80
        self.send_spi_command(SH1106Command::SetMultiplex)?;
        self.send_spi_command(0x3F)?;
        self.send_spi_command(SH1106Command::SetDisplayOffset)?;
        self.send_spi_command(0x0)?; // no offset
        self.send_spi_command((SH1106Command::SetStartLine as u8) | 0x0)?; // line #0
        self.send_spi_command(SH1106Command::ChargePump)?;
        self.send_spi_command(0x14)?;
        self.send_spi_command(SH1106Command::MemoryMode)?;
        self.send_spi_command(0x00)?; // 0x0 act like ks0108
        self.send_spi_command((SH1106Command::SegRemap as u8) | 0x1)?;
        self.send_spi_command(SH1106Command::ComScanDec)?;
        self.send_spi_command(SH1106Command::SetComPins)?;
        self.send_spi_command(0x12)?;
        self.send_spi_command(SH1106Command::SetContrast)?;
        self.send_spi_command(0xCF)?;
        self.send_spi_command(SH1106Command::SetPrecharge)?;
        self.send_spi_command(0xF1)?;
        self.send_spi_command(SH1106Command::SetVComDetect)?;
        self.send_spi_command(0x40)?;
        self.send_spi_command(SH1106Command::DisplayAllOnResume)?;
        self.send_spi_command(SH1106Command::NormalDisplay)?;

        self.send_spi_command(SH1106Command::DisplayOn)
    }

    fn reset(&mut self) -> error::Result<()> {
        self.rst_pin.set_value(1)?;
        thread::sleep(time::Duration::from_millis(1));
        self.rst_pin.set_value(0)?;
        thread::sleep(time::Duration::from_millis(10));
        self.rst_pin.set_value(1)?;
        Ok(())
    }

    fn flip_display(&mut self) {
        self.inverted = !self.inverted;
    }

    fn update(&mut self, bitmap: Bitmap) -> error::Result<()> {
        self.contents = Bitmap::new(self.contents.width(), self.contents.height());
        self.contents.blit(&bitmap, (0, 0));

        // Trim down to the correct size
        self.contents.set_width(self.width);
        self.contents.set_height(self.height);

        let width = self.contents.width() as u8;
        let pages = self.contents.height() / 8;
        let mut data;

        if self.inverted {
            for page in (0..pages).rev() {
                data = vec![];

                // SH1106 has width 132 internally: offset first two columns = 0x02
                self.send_spi_command((SH1106Command::SetHighColumn as u8) | 0x0)?;
                self.send_spi_command((SH1106Command::SetLowColumn as u8) | 0x2)?;
                self.send_spi_command((SH1106Command::SetPage as u8) | 7 - page as u8)?;

                for x in (0..width).rev() {
                    let mut bits: u8 = 0;
                    for bit in 0..8 {
                        bits = bits << 1;
                        bits |= self.contents[page * 8 + bit][x as usize];
                    }
                    data.push(bits);
                }

                self.send_spi_data(&data)?;
            }
        } else {
            for page in 0..pages {
                data = vec![];

                // SH1106 has width 132 internally: offset first two columns = 0x02
                self.send_spi_command((SH1106Command::SetHighColumn as u8) | 0x0)?;
                self.send_spi_command((SH1106Command::SetLowColumn as u8) | 0x2)?;
                self.send_spi_command((SH1106Command::SetPage as u8) | page as u8)?;

                for x in 0..width {
                    let mut bits: u8 = 0;
                    for bit in 0..8 {
                        bits = bits << 1;
                        bits |= self.contents[page * 8 + 7 - bit][x as usize];
                    }
                    data.push(bits);
                }

                self.send_spi_data(&data)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    pub struct TestDisplay {}

    impl Display for TestDisplay {
        fn on(&mut self) -> error::Result<()> {
            Ok(())
        }

        fn reset(&mut self) -> error::Result<()> {
            Ok(())
        }

        fn update(&mut self, _bitmap: Bitmap) -> error::Result<()> {
            Ok(())
        }

        fn flip_display(&mut self) {}

        fn width(&self) -> usize {
            DISPLAY_WIDTH
        }

        fn height(&self) -> usize {
            DISPLAY_HEIGHT
        }
    }
}
