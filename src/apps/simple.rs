use std::error::Error;
use std::time::Duration;

use crossbeam_channel::{Sender, Receiver, bounded};

use crate::control::RoutingDevice;
use crate::messages::{MidiMessage, MessageType};
use crate::routing::Router;
use crate::apps::utility::reset_launchpad;

const DELAY: Duration = Duration::from_millis(100);

pub fn display_pressed(midi_in: &Receiver<MidiMessage>, midi_out: &Sender<MidiMessage>) -> Result<(), Box<Error>>{
    loop {
        match midi_in.recv()? {
            MidiMessage { msg_type: MessageType::CC, key: 111, .. } => {
                break;
            }
            msg => {
                let mut out_msg = msg;
                out_msg.velocity &= 0x60;
                midi_out.send(out_msg)?;
            }
        }
    }

    reset_launchpad(midi_out)
}

pub fn draw_one_color(midi_in: &Receiver<MidiMessage>, midi_out: &Sender<MidiMessage>) -> Result<(), Box<Error>>{
    let router = Router::on_off();
    router.add_input("Launchpad".to_string(), midi_in.clone())?;
    
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
                out_msg.velocity &= 0x03;
                midi_out.send(out_msg)?;
            }
        }

    }
    
    reset_launchpad(midi_out)
}

pub fn rainbow(midi_in: &Receiver<MidiMessage>, midi_out: &Sender<MidiMessage>) -> Result<(), Box<Error>> {
    fn send_note(midi_out: &Sender<MidiMessage>, row: usize, col: usize, color: usize) -> Result<(), Box<Error>> {
        Ok(midi_out.send(
            MidiMessage::new("Launchpad")
                .with_key((row * 16 + col) as u8)
                .with_velocity(color as u8)
                .to_owned()
        )?)
    }

    let mut ending = false;
    let mut offset = 2;

    let colorloop = vec![1, 2, 3, 19, 35, 51, 50, 49, 48, 32, 16, 0];
    let colors = colorloop.len() as isize;

    let mut states = Vec::with_capacity(8);
    for i in 0..8 {
        let mut states_inner: Vec<isize> = Vec::with_capacity(8);
        for j in 0..8 {
            states_inner.push(((i+j) % (2* colors)) as isize - 16);
        }
        states.push(states_inner);
    }

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
                        send_note(midi_out, 7 - row, column, *color)?;
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
                            send_note(midi_out, 7 - row, column, *color)?;
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