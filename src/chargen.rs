// ttxi - chargen.rs
// Copyright (c) 2024 Alistair Buxton <a.j.buxton@gmail.com>
// GPLv3

// Character generator state machine.
// Maintains a 40x25 buffer of teletext bytes
// and updates the terminal when necessary.
// No decoding logic.

use std::io;

use crossterm::{
    execute, queue,
    cursor::{MoveTo, Hide, Show},
    style::{Print, Color, SetBackgroundColor, SetForegroundColor},
    terminal::{
        Clear, EnterAlternateScreen, LeaveAlternateScreen,
        enable_raw_mode, disable_raw_mode, ClearType,
        DisableLineWrap, EnableLineWrap, SetTitle,
        size,
    }
};
use crossterm::style::Attribute::{Hidden, NoBlink, NoHidden, SlowBlink};
use crossterm::style::SetAttribute;

use teletext::{CharacterSet, NationalOption, State};


struct Margins {
    top : u16,
    left : u16,
}


pub struct CharGen {
    grid: [[u8; 40]; 26],
    character_set: CharacterSet,
    mix: bool,
    reveal: bool,
    margins: Margins,
}


impl Drop for CharGen {
    fn drop(&mut self) {
        execute!(io::stdout(),
            Show,
            EnableLineWrap,
            LeaveAlternateScreen,
        ).expect("Error");
        disable_raw_mode().expect("Error");
    }
}

impl CharGen {
    pub fn new() -> io::Result<CharGen> {
        let (cols, rows) = size()?;
        if cols < 41 || rows < 25 {
            return Err(io::Error::new(io::ErrorKind::Other, "Terminal too small - 41x25 required."))
        }

        let chargen = CharGen {
            grid: [[0; 40]; 26],
            character_set: CharacterSet::Latin(NationalOption::English),
            mix: false,
            reveal: false,
            margins: Margins{top: 0, left: 0},
        };

        enable_raw_mode()?;
        execute!(io::stdout(),
            EnterAlternateScreen,
            Clear(ClearType::All),
            DisableLineWrap,
            Hide,
            SetTitle("Teletext"),
        )?;
        Ok(chargen)
    }

    pub fn auto_margins(&mut self) -> io::Result<()> {
        let (cols, rows) = size()?;
        self.margins.left = if cols < 41 {
             0
        } else {
            (cols - 41) / 2
        };
        self.margins.top = if rows < 26 {
             0
        } else {
            (rows - 26) - ((rows - 26) / 2)
        };
        self.redraw()
    }

    #[allow(dead_code)]
    pub fn set_margins(&mut self, top : u16, left : u16) -> io::Result<()> {
        self.margins.left = left;
        self.margins.top = top;
        self.redraw()
    }

    fn default_bg(&mut self) -> Color {
        if self.mix {
            Color::Reset
        } else {
            self.term_color(0)
        }
    }

    #[allow(dead_code)]
    pub fn clear_all(&mut self) -> io::Result<()> {
        for row in 0..26
        {
            self.grid[row] = [0; 40];
        }
        execute!(io::stdout(),
            SetBackgroundColor(self.default_bg()),
            Clear(ClearType::All),
        )
    }

    #[allow(dead_code)]
    pub fn clear_page(&mut self) -> io::Result<()> {
        queue!(io::stdout(),
            SetBackgroundColor(self.default_bg()),
        )?;
        for row in 1..25
        {
            self.grid[row] = [0; 40];
            queue!(io::stdout(),
                MoveTo(0, self.margins.top + row as u16),
                Clear(ClearType::CurrentLine),
            )?;
        }
        Ok(())
    }

    pub fn redraw(&mut self) -> io::Result<()> {
        queue!(io::stdout(),
            SetBackgroundColor(self.default_bg()),
            Clear(ClearType::All),
        )?;
        for row in 0..25
        {
            self.render_row(row)?;
        }
        Ok(())
    }

    pub fn reveal(&mut self) -> io::Result<()> {
        self.reveal = !self.reveal;
        self.redraw()
    }

    pub fn mix(&mut self) -> io::Result<()> {
        self.mix = !self.mix;
        self.redraw()
    }

    fn term_color(&self, color: u8) -> Color {
/*
        match color {
            0 => Color::Black,
            1 => Color::Red,
            2 => Color::Green,
            3 => Color::Yellow,
            4 => Color::Blue,
            5 => Color::Magenta,
            6 => Color::Cyan,
            _ => Color::White,
        }
*/
        match color {
            0 => Color::Rgb{r:0, g:0, b:0},
            1 => Color::Rgb{r:255, g:0, b:0},
            2 => Color::Rgb{r:0, g:255, b:0},
            3 => Color::Rgb{r:255, g:255, b:0},
            4 => Color::Rgb{r:0, g:0, b:255},
            5 => Color::Rgb{r:255, g:0, b:255},
            6 => Color::Rgb{r:0, g:255, b:255},
            _ => Color::Rgb{r:255, g:255, b:255},
        }
    }

    pub fn render_row(&self, row: u8) -> io::Result<()> {
        let mut out = std::io::stdout();
        let mut current_fg = self.term_color(1);
        let mut current_bg = if self.mix {
            Color::Reset
        } else {
            self.term_color(0)
        };
        let mut current_conceal = false;
        let mut current_flash = false;
        let mut state = State::new();
        queue!(out, MoveTo(0, self.margins.top + row as u16), SetForegroundColor(current_fg), SetBackgroundColor(current_bg), Print(" ".repeat(self.margins.left as usize)))?;
        for byte in &self.grid[row as usize] {
            let (element, new_state) = state.next(*byte);

            let bg = if self.mix && !element.style.boxed {
                Color::Reset
            } else {
                self.term_color(element.style.background)
            };
            if bg != current_bg {
                queue!(out, SetBackgroundColor(bg))?;
                current_bg = bg;
            }

            let fg = self.term_color(element.style.foreground);
            if fg != current_fg {
                queue!(out, SetForegroundColor(fg))?;
                current_fg = fg;
            }

            let conceal = element.style.conceal && ! self.reveal;
            if conceal != current_conceal {
                if conceal {
                    queue!(out, SetAttribute(Hidden))?;
                }else {
                    queue!(out, SetAttribute(NoHidden))?;
                }
                current_conceal = conceal;
            }

            let flash = element.style.flash;
            if flash != current_flash {
                if flash {
                    queue!(out, SetAttribute(SlowBlink))?;
                }else {
                    queue!(out, SetAttribute(NoBlink))?;
                }
                current_flash = flash;
            }

            let c = teletext::to_char(&element, self.character_set).to_string();
            queue!(out, Print(c))?;
            state = new_state;
        }

        let bg = if self.mix {
            Color::Reset
        } else {
            self.term_color(0)
        };

        execute!(out, SetBackgroundColor(bg), Print("     "))
    }

    pub fn insert_data(&mut self, row : u8, col : u8, data : &[u8]) -> io::Result<()> {
        if &self.grid[row as usize][col as usize..data.len() + col as usize] != data {
            self.grid[row as usize][col as usize..data.len() + col as usize].copy_from_slice(data);
            self.render_row(row)
        } else {
            Ok(())
        }
    }
}