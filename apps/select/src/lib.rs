use std::error::Error;

use midichan_core::message::{MidiMessage, MessageType};
use midichan_core::device::Application;
use launchpad::{Launchpad, Color};


pub struct Select {
    choices: Vec<Box<dyn Application>>,
    launchpad: Launchpad
}

fn midi_to_item(msg: &MidiMessage) -> usize {
    let row = msg.key / 16;
    let col = msg.key % 16;
    if col >= 8 || msg.msg_type == MessageType::CC {
        255
    } else {
        (row * 8 + col) as usize
    }
}

impl Select {
    pub fn new(launchpad: Launchpad) -> Select {
        Select{choices: Vec::new(), launchpad: launchpad}
    }

    pub fn add(&mut self, choice: Box<dyn Application>) {
        self.choices.push(choice);
    }

    pub fn display_choices(&self) -> Result<(), Box<dyn Error>> {
        self.launchpad.clear()?;
        self.launchpad.set(7, 8, &Color::new(3, 0))?;

        for item in 0..self.choices.len() {
            self.launchpad.set(item as u8 % 8, item as u8 / 8, &Color::new(3, 3))?;
        }

        Ok(())
    }
}

impl Application for Select {
    fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let midi_in = self.launchpad.input();
        loop {
            self.display_choices()?;

            match midi_in.recv()? {
                MidiMessage { msg_type: MessageType::CC, key: 111, velocity: vel, .. } => {
                    if vel == 127 {
                        self.launchpad.clear()?;
                        break;
                    }
                },

                msg => {
                    if msg.velocity == 127 {
                        if let Some(x) = self.choices.get_mut(midi_to_item(&msg)) {
                            self.launchpad.clear()?;

                            x.run()?;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}