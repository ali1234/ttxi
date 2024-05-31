// ttxi - decoder.rs
// Copyright (c) 2024 Alistair Buxton <a.j.buxton@gmail.com>
// GPLv3

// Programmable Teletext packet decoder.

use std::io;
use crate::chargen::CharGen;
use crate::coding::{ascii_to_u8, control_bits, hamming16_decode, mrag, u8_to_ascii};
use crate::keymap::{Button};


pub struct PageInput {
    pub mag : u8,
    pub page : u8,
    pub hold : bool,
    manual_hold : bool,
    pub input : [u8; 6],
    input_pos : usize,
}

impl Default for PageInput {
    fn default() -> PageInput {
        PageInput {
            mag: 1,
            page: 0,
            hold: false,
            manual_hold: false,
            input : [b' ', b'P', b'1', b'0', b'0', b'\x02'],
            input_pos: 2,
        }
    }
}

impl PageInput {
    fn input(&mut self, b : u8) {
        if self.manual_hold {
            self.cancel();
        }

        match (b, self.input_pos) {
            (1..=8, 2) => {
                self.input[0] = b'\x02';
                self.input[1] = b'P';
                self.input[2] = u8_to_ascii(b);
                self.input[3] = b'.';
                self.input[4] = b'.';
                self.input[5] = b'\x07';
                self.input_pos += 1;
                self.hold = true;
            }
            (0..=0xf, 3 | 4) => {
                self.input[self.input_pos] = u8_to_ascii(b);
                self.input_pos += 1;
                if self.input_pos == 5 {
                    self.input[0] = b' ';
                    self.input[5] = b'\x02';
                    self.input_pos = 2;
                    self.mag = ascii_to_u8(self.input[2]) & 0x7;
                    self.page = (ascii_to_u8(self.input[3]) << 4) | ascii_to_u8(self.input[4]);
                    self.hold = false;
                }
            }
            _ => {}
        }
    }

    pub(crate) fn cancel(&mut self) {
        self.input[0] = b' ';
        self.input[1] = b'P';
        self.input[2] = u8_to_ascii(self.mag);
        self.input[3] = u8_to_ascii(self.page >> 4);
        self.input[4] = u8_to_ascii(self.page & 0xf);
        self.input[5] = b'\x02';
        self.input_pos = 2;
    }

    fn toggle_hold(&mut self) {
        self.manual_hold = ! self.manual_hold;
        self.hold = self.manual_hold;
        if !self.hold {
            self.cancel();
        } else {
            self.input[0] = b'\x01';
            self.input[1] = b'H';
            self.input[2] = b'O';
            self.input[3] = b'L';
            self.input[4] = b'D';
            self.input[5] = b'\x07';
        }
    }
}



pub struct Decoder {
    pub header_matched: bool,
    pub header_locked: bool,
    pub pageinput: PageInput,
    pub chargen: CharGen,
}

impl Decoder {

    pub fn new() -> io::Result<Decoder> {
        let mut decoder = Decoder {
            header_matched: false,
            header_locked: false,
            pageinput: Default::default(),
            chargen: CharGen::new()?,
        };

        decoder.chargen.insert_data(0, 2, &decoder.pageinput.input)?;
        decoder.chargen.auto_margins()?;
        Ok(decoder)
    }

    pub fn process_packet(&mut self, packet: [u8; 42]) -> io::Result<()> {
        let (mag, row) = mrag(&packet[..2]);
        if !self.pageinput.hold && mag == self.pageinput.mag {
            match row {
                0 => {
                    let page = hamming16_decode(&packet[2..4]);
                    self.header_matched = page == self.pageinput.page;
                    if self.header_locked {
                        self.chargen.insert_data(0, 31, &packet[33..])?;
                    } else {
                        self.chargen.insert_data(0, 8, &packet[10..])?;
                    }
                    if self.header_matched {
                        self.header_locked = true;
                        self.chargen.insert_data(0, 7, &[b'\x07'])?;
                        let control = control_bits(&packet[4..12]);
                        if (control & 0x0001) > 0 {
                            self.chargen.clear_page()?;
                        }
                    }
                    // todo: clear page if flags set
                    // todo: set mix if flags set
                }
                1..=24 => {
                    if self.header_matched {
                        self.chargen.insert_data(row, 0, &packet[2..])?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn process_button(&mut self, button: Button) -> io::Result<()> {
        match button {
            Button::Digit(x) => {
                self.pageinput.input(x);// numbers
                self.header_locked = false;
                self.chargen.insert_data(0, 2, &self.pageinput.input)
            }
            Button::Fastext(_i) => Ok(()),
            Button::PageNext => Ok(()),
            Button::PagePrev => Ok(()),
            Button::Hold => {
                self.pageinput.toggle_hold();
                self.chargen.insert_data(0, 2, &self.pageinput.input)?;
                if !self.pageinput.hold {
                    self.header_locked = false;
                    self.chargen.insert_data(0, 7, &[b'\x02'])?;
                }
                Ok(())
            }
            Button::Reveal => self.chargen.reveal(),
            Button::Mix => self.chargen.mix(),
            Button::TimedPage => Ok(()),
        }
    }
}