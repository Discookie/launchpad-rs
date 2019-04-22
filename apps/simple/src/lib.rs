#[macro_use]
extern crate crossbeam_channel;

use std::error::Error;
use std::time::Duration;

use crossbeam_channel::{Sender, Receiver, bounded};

use midichan_core::device::{RoutingDevice, Application};
use midichan_core::message::{MidiMessage, MessageType};
use router::Router;
use launchpad::{Launchpad, Color};

const DELAY: Duration = Duration::from_millis(100);

pub struct DisplayPressed {
    color: Color,
    input: Receiver<MidiMessage>,
    output: Sender<MidiMessage>
}

impl DisplayPressed {
    pub fn new(midi_in: Receiver<MidiMessage>, midi_out: Sender<MidiMessage>) -> DisplayPressed {
        DisplayPressed{color: Color::new(0, 1), input: midi_in, output: midi_out}
    }

    pub fn with_color(&mut self, color: &Color) -> &mut DisplayPressed {
        self.color = color.clone();
        self
    }
}

impl Application for DisplayPressed {
    fn run(&mut self) -> Result<(), Box<Error>> {
        loop {
            match self.input.recv()? {
                MidiMessage { msg_type: MessageType::CC, key: 111, .. } => {
                    break;
                }
                msg => {
                    let mut out_msg = msg;
                    out_msg.velocity &= self.color.color();
                    self.output.send(out_msg)?;
                }
            }
        }

        Ok(())
    }
}

pub struct DrawOneColor {
    color: Color,
    input: Receiver<MidiMessage>,
    output: Sender<MidiMessage>
}

impl DrawOneColor {
    pub fn new(midi_in: Receiver<MidiMessage>, midi_out: Sender<MidiMessage>) -> DrawOneColor {
        DrawOneColor{color: Color::new(3, 0), input: midi_in, output: midi_out}
    }

    pub fn with_color(&mut self, color: &Color) -> &mut DrawOneColor {
        self.color = color.clone();
        self
    }
}

impl Application for DrawOneColor {
    fn run(&mut self) -> Result<(), Box<Error>> {
        let router = Router::on_off();
        router.add_input("DrawInput".to_string(), self.input.clone())?;

        let (midi_send, midi_recv) = bounded(128);

        router.add_output("on".to_string(), midi_send.clone())?;

        loop {
            match midi_recv.recv()? {
                MidiMessage { msg_type: MessageType::CC, key: 111, .. } => {
                    break;
                },

                MidiMessage { msg_type: MessageType::CC, key: 110, .. } => {
                    router.remove_output("on".to_string())?;
                    router.add_output("off".to_string(), midi_send.clone())?;
                },

                MidiMessage { msg_type: MessageType::CC, key: 109, .. } => {
                    router.remove_output("off".to_string())?;
                    router.add_output("on".to_string(), midi_send.clone())?;
                },

                msg => {
                    let mut out_msg = msg;
                    out_msg.velocity &= self.color.color();
                    self.output.send(out_msg)?;
                }
            }
        }
        Ok(())
    }
}

pub struct Rainbow {
    launchpad: Launchpad
}

impl Rainbow {
    pub fn new(launchpad: Launchpad) -> Rainbow {
        Rainbow{launchpad: launchpad}
    }
}

impl Application for Rainbow {
    fn run(&mut self) -> Result<(), Box<Error>> {
        let mut ending = false;
        let mut offset = 2;

        let colorloop = vec![
            Color::new(1, 0),
            Color::new(2, 0),
            Color::new(3, 0),
            Color::new(3, 1),
            Color::new(3, 2),
            Color::new(3, 3),
            Color::new(2, 3),
            Color::new(1, 3),
            Color::new(0, 3),
            Color::new(0, 2),
            Color::new(0, 1),
            Color::new(0, 0)];
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

        loop {
            select!{
                recv(midi_in) -> msg => match msg {
                    Ok(MidiMessage { velocity: 127, .. }) => ending = true,
                    _ => {}
                },

                default(DELAY) => {
                    for row in 0..8 {
                        let column = (row*3 + offset) % 5;
                        states[row][column] += 1; // tick

                        if !ending || states[7][7] < 2 * (colors - 1) { // we are not finished
                            states[row][column] %= 2 * colors;
                        } else {
                            states[row][column].min(2 * (colors-1));
                        }

                        if let Some(color) = colorloop.get(states[row][column] as usize / 2) {
                            self.launchpad.set(7 - row as u8, column as u8, color)?;
                        }

                        if column < 3 {
                            let column = column + 5;

                            states[row][column] += 1; // tick

                            if !ending || states[7][7] < 2 * (colors - 1) { // we are not finished
                                states[row][column] %= 2 * colors;
                            } else {
                                states[row][column].min(2 * (colors-1));
                            }

                            if let Some(color) = colorloop.get(states[row][column] as usize / 2) {
                                self.launchpad.set(7 - row as u8, column as u8, color)?;
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