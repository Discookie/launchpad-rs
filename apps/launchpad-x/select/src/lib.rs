use std::error::Error;

use midichan_core::message::{MidiMessage, MessageType};
use midichan_core::device::Application;
use launchpad_x::*;


pub struct Select {
    choices: Vec<(String, Box<dyn Application>)>,
    launchpad: LaunchpadX,
    text_mode: bool
}

fn midi_to_item(msg: &MidiMessage) -> usize {
    let row = msg.key / 10;
    let col = msg.key % 10;
    if col > 8 || row == 0 || col == 0 || msg.msg_type == MessageType::CC {
        255
    } else {
        ((row-1) * 8 + (col-1)) as usize
    }
}

impl Select {
    pub fn new(launchpad: LaunchpadX) -> Select {
        Select {
            choices: Vec::new(),
            launchpad,
            text_mode: false
        }
    }

    pub fn add(&mut self, name: String, choice: Box<dyn Application>) {
        self.choices.push((name, choice));
    }

    pub fn display_choices(&mut self) -> Result<(), Box<dyn Error>> {
        self.launchpad.clear_daw_state(true, false, false)?;

        for item in 0..self.choices.len() {
            self.launchpad.set_session(item as u8 % 8, item as u8 / 8, Color { color: 21, pulse_mode: PulseMode::Static })?;
        }

        // Exit
        self.launchpad.set_session(7, 8, Color { color: 6, pulse_mode: PulseMode::Static })?;
        // Logo
        self.launchpad.set_session(8, 8, Color { color: 34, pulse_mode: PulseMode::Static })?;
        // Text display
        self.launchpad.set_session(8, 7, Color { 
            color: if self.text_mode { 14 } else { 11 },
            pulse_mode: PulseMode::Static
        })?;

        Ok(())
    }
}

impl Application for Select {
    fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let midi_in = self.launchpad.daw_input();
        self.launchpad.set_screen(LaunchpadScreen::Session)?;

        std::thread::sleep(std::time::Duration::from_millis(100));
        self.display_choices()?;

        loop {

            match midi_in.recv()? {
                MidiMessage { msg_type: MessageType::CC, key: 98, velocity: vel, .. } if vel > 0 => {
                    self.launchpad.clear()?;
                    break;
                },
                MidiMessage { msg_type: MessageType::CC, key: 89, velocity: vel, .. } if vel > 0 => {
                    self.text_mode = !self.text_mode;
                    self.display_choices()?;
                },

                msg if msg.velocity > 0 => {
                    
                    if let Some((name, x)) = self.choices.get_mut(midi_to_item(&msg)) {
                        if self.text_mode {
                            self.launchpad.scroll_text(&name, Color { color: 21, pulse_mode: PulseMode::Static }, 10, false)?;
                        } else {
                            self.launchpad.clear_daw_state(true, false, false)?;
                            std::thread::sleep(std::time::Duration::from_millis(300));
                            x.run()?;
                            self.display_choices()?;
                        }
                    }
                },

                _ => ()
            }
        }

        Ok(())
    }
}