// ttxi - main.rs
// Copyright (c) 2024 Alistair Buxton <a.j.buxton@gmail.com>
// GPLv3

mod decoder;
mod keymap;
mod chargen;
mod coding;

use std::io;
use std::fs::File;
use std::sync::mpsc::{self, Receiver, SendError};
use std::{thread};
use std::net::SocketAddr;


use crossterm::event::{Event, KeyCode, KeyEvent};
use socket2::{Socket, Domain, Type};

use crate::decoder::{Decoder};
use crate::keymap::Button;


enum MyEvent {
    Input(Event),
    Packet([u8; 42]),
    EndOfStream(()),
    EndOfInput(()),
}


fn spawn_event_channel(source: String) -> io::Result<Receiver<MyEvent>> {
    let (tx_packet, rx) = mpsc::sync_channel::<MyEvent>(0);
    let tx_input = tx_packet.clone();

    if let Some(addr) = source.strip_prefix("udp://") {
        let stream = Socket::new(Domain::IPV4, Type::DGRAM, None)?;
        stream.set_reuse_address(true)?;
        let sa: SocketAddr = addr.parse().expect("Could not parse UDP address.");
        stream.bind(&sa.into())?;
        let mut buffer: Vec<u8> = Vec::with_capacity(65536);
        thread::spawn(move || loop {
            let (amt, _src) = stream.recv_from(buffer.spare_capacity_mut()).unwrap();
            unsafe { buffer.set_len(amt); }
            if (amt % 42) == 0 {
                for chunk in buffer.chunks_exact(42) {
                    if tx_packet.send(MyEvent::Packet(chunk.try_into().unwrap())).is_err() {
                        return
                    }
                }
            }
            buffer.clear();
        });
    } else {
        let mut stream : Box<dyn io::Read + Send> = if let Some(addr) = source.strip_prefix("tcp://") {
            let sa: SocketAddr = addr.parse().expect("Could not parse TCP address.");
            let stream = Box::new(Socket::new(Domain::IPV4, Type::STREAM, None)?);
            stream.connect(&sa.into())?;
            stream
        } else {
            Box::new( File::open(source)? )
        };

        thread::spawn(move || loop {
            // TODO: use read_buf_exact when it is stable.
            let mut buffer: [u8; 42] = [0; 42];
            match stream.read_exact(&mut buffer) {
                Ok(()) => {
                    tx_packet.send(MyEvent::Packet(buffer))?;
                }
                Err(_) => {
                    tx_packet.send(MyEvent::EndOfStream(()))?;
                    break Ok::<(), SendError<MyEvent>>(());
                }
            }
        });
    }

    thread::spawn(move || loop {
        match crossterm::event::read() {
            Ok(event) => {
                tx_input.send(MyEvent::Input(event))?;
            }
            Err(_) => {
                tx_input.send(MyEvent::EndOfInput(()))?;
                break Ok::<(), SendError<MyEvent>>(());
            }
        }
    });

    Ok(rx)
}


fn main() -> io::Result<()> {
    let source = std::env::args().nth(1).expect("No source given");
    let event_channel = spawn_event_channel(source)?;

    let mut decoder = Decoder::new()?;

    loop {
        match event_channel.recv() {
            Ok(MyEvent::Packet(packet)) => decoder.process_packet(packet)?,
            Ok(MyEvent::Input(Event::Key(ev))) => match ev {
                KeyEvent{code: KeyCode::Char('q'), ..} => { break Ok (()); }
                _ => match Button::from_event(ev) {
                    Some(button) => decoder.process_button(button)?,
                    None => {}
                }
            }
            Ok(MyEvent::Input(Event::Resize(_, _))) => { decoder.chargen.auto_margins()?; }
            Ok(MyEvent::Input(_)) => {}
            _ => { break Ok(()); }
        }
    }
}