use std::error::Error;

use crossbeam_channel::{Sender, Receiver};

use crate::messages::{MidiMessage, MessageType};
use crate::apps::simple::*;
use crate::apps::utility::reset_launchpad;


fn midi_to_item(msg: &MidiMessage) -> usize {
    let row = msg.key / 16;
    let col = msg.key % 16;
    if col >= 8 || msg.msg_type == MessageType::CC {
        255
    } else {
        (row * 8 + col) as usize
    }
}
fn item_to_midi(item: usize) -> u8 {
    let row = item / 8;
    let col = item % 8;
    (row * 16 + col) as u8
}

fn display_choices(midi_out: &Sender<MidiMessage>, choices: usize) -> Result<(), Box<Error>> {
    reset_launchpad(midi_out)?;
    midi_out.send(
        MidiMessage::new("Launchpad")
            .with_msg_type(MessageType::CC)
            .with_key(0x6F)
            .with_velocity(3).to_owned())?;

    for item in 0..choices {
        midi_out.send(
            MidiMessage::new("Launchpad")
                .with_key(item_to_midi(item))
                .with_velocity(0x63).to_owned())?;
    }
    Ok(())
}

pub fn select(midi_in: &Receiver<MidiMessage>, midi_out: &Sender<MidiMessage>) -> Result<(), Box<Error>> {
    let apps = vec!["display_pressed", "draw_one_color", "rainbow"];

    loop {
        display_choices(midi_out, apps.len())?;

        match midi_in.recv()? {
            MidiMessage { msg_type: MessageType::CC, key: 111, velocity: 127, .. } => {
                break;
            },

            msg => {
                if msg.velocity == 127 {
                    match apps.get(midi_to_item(&msg)) {
                        Some(x) => {
                            reset_launchpad(midi_out)?;

                            match x {
                                &"display_pressed" => display_pressed(midi_in, midi_out)?,
                                &"draw_one_color" => draw_one_color(midi_in, midi_out)?,
                                &"rainbow" => rainbow(midi_in, midi_out)?,
                                _ => {}
                            }
                        },
                        _ => {}
                    }
                }
            }
        }
    }
    
    reset_launchpad(midi_out)
}