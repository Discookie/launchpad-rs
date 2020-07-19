#[macro_use]
extern crate crossbeam_channel;

use std::error::Error;
use std::time::Duration;

use crossbeam_channel::bounded;

use midichan_core::device::{RoutingDevice, Application};
use midichan_core::message::{MidiMessage, MessageType};
use router::Router;
use launchpad_x::*;

const DELAY: Duration = Duration::from_millis(50);

pub struct DisplayPressed {
    color: Color,
    launchpad: LaunchpadX
}

impl DisplayPressed {
    pub fn new(launchpad: LaunchpadX) -> DisplayPressed {
        DisplayPressed {
            color: Color {
                color: 37,
                pulse_mode: PulseMode::Pulse
            },
            launchpad
        }
    }

    pub fn with_color(&mut self, color: Color) -> &mut DisplayPressed {
        self.color = color;
        self
    }
}

impl Application for DisplayPressed {
    fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let input = self.launchpad.input();
        let output = self.launchpad.output();
        self.launchpad.set_programmer_mode(true)?;
        
        loop {
            match input.recv()? {
                MidiMessage { msg_type: MessageType::CC, key: 98, .. } => {
                    self.launchpad.set_programmer_mode(false)?;
                    break;
                },
                msg if matches!(msg.msg_type, MessageType::NoteOn | MessageType::CC) => {
                    let mut out_msg = msg;
                    if out_msg.velocity > 0 {
                        out_msg.velocity = self.color.color;
                        out_msg.channel = self.color.pulse_mode as u8;
                    } else {
                        out_msg.velocity = 0;
                        out_msg.channel = 0;
                    }

                    output.send(out_msg)?;
                },
                _ => ()
            }
        }

        Ok(())
    }
}

pub struct DrawOneColor {
    color: Color,
    launchpad: LaunchpadX
}

impl DrawOneColor {
    pub fn new(launchpad: LaunchpadX) -> DrawOneColor {
        DrawOneColor {
            color: Color {
                color: 84,
                pulse_mode: PulseMode::Static
            },
            launchpad
        }
    }

    pub fn with_color(&mut self, color: &Color) -> &mut DrawOneColor {
        self.color = color.clone();
        self
    }
}

impl Application for DrawOneColor {
    fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let router = Router::on_off();
        router.add_input("DrawInput".to_string(), self.launchpad.input())?;

        let (midi_send, midi_recv) = bounded(128);

        router.add_output("on".to_string(), midi_send.clone())?;

        let output = self.launchpad.output();
        self.launchpad.set_programmer_mode(true)?;

        loop {
            match midi_recv.recv()? {
                MidiMessage { msg_type: MessageType::CC, key: 98, .. } => {
                    self.launchpad.set_programmer_mode(false)?;
                    break;
                },

                MidiMessage { msg_type: MessageType::CC, key: 97, .. } => {
                    router.remove_output("on".to_string())?;
                    router.add_output("off".to_string(), midi_send.clone())?;
                },

                MidiMessage { msg_type: MessageType::CC, key: 96, .. } => {
                    router.remove_output("off".to_string())?;
                    router.add_output("on".to_string(), midi_send.clone())?;
                },

                msg if matches!(msg.msg_type, MessageType::NoteOn | MessageType::CC) => {
                    let mut out_msg = msg;
                    if out_msg.velocity > 0 {
                        out_msg.velocity = self.color.color;
                        out_msg.channel = self.color.pulse_mode as u8;
                    } else {
                        out_msg.velocity = 0;
                        out_msg.channel = 0;
                    }

                    output.send(out_msg)?;
                },
                _ => ()
            }
        }
        Ok(())
    }
}

pub struct Rainbow {
    launchpad: LaunchpadX
}

impl Rainbow {
    pub fn new(launchpad: LaunchpadX) -> Rainbow {
        Rainbow { launchpad }
    }
}

impl Application for Rainbow {
    fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let mut ending = false;
        let mut offset = 2;

        macro_rules! color {
            ($num: literal) => { Color { color: $num, pulse_mode: PulseMode::Static } }
        }
        let colorloop = vec![
            color!(7),
            color!(10),
            color!(14),
            color!(17),
            color!(21),
            color!(25),
            color!(29),
            color!(33),
            color!(37),
            color!(41),
            color!(45),
            color!(49),
            color!(54),
            color!(59),
            color!(0),
            color!(0),
        ];
        let colors = colorloop.len() as isize;

        let mut states = Vec::with_capacity(8);
        for i in 0..8 {
            let mut states_inner: Vec<isize> = Vec::with_capacity(8);
            for j in 0..8 {
                states_inner.push(((i+j) % (2* colors)) as isize - 16);
            }
            states.push(states_inner);
        }

        let midi_in = self.launchpad.input();
        self.launchpad.set_programmer_mode(true)?;

        loop {
            select!{
                recv(midi_in) -> msg => match msg {
                    Ok(MidiMessage { velocity: vel, .. }) if vel > 0 => ending = true,
                    _ => {}
                },

                default(DELAY) => {
                    for row in 0..8 {
                        let column = (row*3 + offset) % 5;
                        states[row][column] += 1; // tick

                        if !ending || states[7][7] < 2 * (colors - 1) { // we are not finished
                            states[row][column] %= 2 * colors;
                        } else {
                            states[row][column] = states[row][column].min(2 * (colors-1));
                        }

                        if let Some(color) = colorloop.get(states[row][column] as usize / 2) {
                            self.launchpad.set(7 - row as u8, column as u8, *color)?;
                        }

                        if column < 3 {
                            let column = column + 5;

                            states[row][column] += 1; // tick

                            if !ending || states[7][7] < 2 * (colors - 1) { // we are not finished
                                states[row][column] %= 2 * colors;
                            } else {
                                states[row][column] = states[row][column].min(2 * (colors-1));
                            }

                            if let Some(color) = colorloop.get(states[row][column] as usize / 2) {
                                self.launchpad.set(7 - row as u8, column as u8, *color)?;
                            }
                        }
                    }

                    if ending {
                        let mut finished = true;

                        'ending: for states_inner in states.iter() {
                            for elem in states_inner.iter() {
                                if elem < &(2 * (colors - 1)) && elem > &0 {
                                    finished = false;
                                    break 'ending;
                                }
                            }
                        }

                        if finished {
                            self.launchpad.set_programmer_mode(false)?;
                            break;
                        }
                    }

                    offset += 1;
                }
            }
        }
        Ok(())
    }
}