// ttxi - keymap.rs
// Copyright (c) 2024 Alistair Buxton <a.j.buxton@gmail.com>
// GPLv3


use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};


pub enum Button {
    Digit(u8),
    Fastext(u8),
    PageNext,
    PagePrev,
    Hold,
    Reveal,
    TimedPage,
    Mix,
}

impl Button {
    pub fn from_event(key_event : KeyEvent) -> Option<Button> {
        match key_event.modifiers {
            KeyModifiers::SHIFT => match key_event.code {
                KeyCode::Char(c @ '0'..='6') => Some(Button::Fastext(c.to_digit(16).unwrap() as u8)),
                _ => None,
            }
            KeyModifiers::NONE => match key_event.code {
                KeyCode::Char(c @ ('0'..='9' | 'a'..='f')) => Some(Button::Digit(c.to_digit(16).unwrap() as u8)),

                KeyCode::Char('h') | KeyCode::Char(' ') => Some(Button::Hold),
                KeyCode::Char('r') => Some(Button::Reveal),
                KeyCode::Char('t') => Some(Button::TimedPage),

                KeyCode::Up | KeyCode::Right | KeyCode::PageUp => Some(Button::PageNext),
                KeyCode::Down | KeyCode::Left | KeyCode::PageDown => Some(Button::PagePrev),

                KeyCode::Char('m') => Some(Button::Mix),

                _ => None,
            }
            _ => None,
        }
    }
}
