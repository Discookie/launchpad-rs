use std::error::Error;
use std::cmp::{min};

use crossbeam_channel::{Sender, Receiver};
use midichan_core::message::{MidiMessage, MessageType};

#[derive(Clone)]
pub struct Color {
    val: u8
}

impl Color {
    pub fn color(&self) -> u8 {
        self.val
    }

    pub fn new(red: u8, green: u8) -> Color {
        Color{val: red + green * 0x10}
    }

    pub fn with_color(&mut self, red: u8, green: u8) -> &mut Color {
        self.val = self.val & !0x63 + red + green*0x20;
        self
    }
}

#[derive(Clone)]
pub struct Launchpad {
    name: String,
    input: Receiver<MidiMessage>,
    output: Sender<MidiMessage>
}

impl Launchpad {
    pub fn new(in_port: Receiver<MidiMessage>, out_port: Sender<MidiMessage>) -> Launchpad {
        Launchpad{
            name: "Launchpad".to_string(),
            input: in_port,
            output: out_port
        }
    }

    pub fn with_name(&mut self, name: String) -> &mut Launchpad {
        self.name = name;
        self
    }

    pub fn input(&self) -> Receiver<MidiMessage> {
        self.input.clone()
    }

    pub fn output(&self) -> Sender<MidiMessage> {
        self.output.clone()
    }

    pub fn clear(&self) -> Result <(), Box<dyn Error>> {
        Ok(self.output.send(
            MidiMessage{
                device: self.name.clone(),
                timestamp: 0,
                channel: 0,
                msg_type: MessageType::CC,
                key: 0,
                velocity: 0,
                sysex: None
            }
        )?)
    }

    pub fn set(&self, x: u8, y: u8, color: &Color) -> Result<(), Box<dyn Error>> {
        Ok(self.output.send(
            MidiMessage{
                device: self.name.clone(),
                timestamp: 0,
                channel: 0,
                msg_type: match y {
                    8 => MessageType::CC,
                    _ => MessageType::NoteOn
                },
                key: match y {
                    8 => 0x68 + x,
                    _ => y * 0x10 + x
                },
                velocity: color.color(),
                sysex: None
            }
        )?)
    }

    pub fn fill_step(&self, first: &Color, second: &Color) -> Result<(), Box<dyn Error>> {
        Ok(self.output.send(
            MidiMessage{
                device: self.name.clone(),
                timestamp: 0,
                channel: 5,
                msg_type: MessageType::NoteOn,
                key: first.color(),
                velocity: second.color(),
                sysex: None
            }
        )?)
    }

    pub fn fill(&self, grid: Vec<Vec<Color>>) -> Result<(), Box<dyn Error>> {
        let mut temp = Color::new(0, 0);
        let mut has_val = false;
        for x_ind in 0..grid.len() {
            for y_ind in 0..min(grid[x_ind].len(), 8) {
                if has_val {
                    self.fill_step(&temp, &grid[x_ind][y_ind])?;
                } else {
                    temp = grid[x_ind][y_ind].clone();
                }
                has_val = !has_val;
            }

            for _ in grid[x_ind].len()..8 {
                if has_val {
                    self.fill_step(&temp, &Color::new(0, 0))?;
                } else {
                    temp = Color::new(0, 0);
                }
                has_val = !has_val;
            }
        }

        // TODO Sidebar and top bar

        Ok(())
    }
}