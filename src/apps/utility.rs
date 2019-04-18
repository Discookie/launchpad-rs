
use std::error::Error;

use crossbeam_channel::{Sender, Receiver};

use crate::messages::{MidiMessage, MessageType};


pub fn reset_launchpad(midi_out: &Sender<MidiMessage>) -> Result<(), Box<Error>> {
    Ok(midi_out.send(MidiMessage{
        device: "Launchpad".to_string(),
        timestamp: 0,
        channel: 0,
        msg_type: MessageType::CC,
        key: 0,
        velocity: 0
    })?)
}

pub fn direct_route(midi_in: &Receiver<MidiMessage>, midi_out: &Sender<MidiMessage>) -> Result<(), Box<Error>> {
    loop {
        select!{
            recv(midi_in) -> msg => 
                midi_out.send(msg?)?,
        }
    }
}