#[macro_use]
extern crate crossbeam_channel;
extern crate midir;
extern crate smallvec;

#[macro_use]
mod utils;
mod device;
mod routing;
mod control;
mod messages;
mod apps;


use device::{InputDevice, OutputDevice};
use control::{MidiDevice};

use apps::select::select;


const IN_PORT:  usize = 0;    
const OUT_PORT: usize = 1;




fn main() {
    let in_device = InputDevice::start();
    let out_device = OutputDevice::start();

    in_device.open_port("Launchpad".to_string(), IN_PORT)
        .unwrap_or_else(|x| println!("opening errored: {}", x));
    out_device.open_port("Launchpad".to_string(), OUT_PORT)
        .unwrap_or_else(|x| println!("opening errored: {}", x));

    select(&in_device.midi_in(), &out_device.midi_out())
        .unwrap_or_else(|x| println!("routing errored: {}", x));

    // draw_one_color(in_device.midi_in(), out_device.midi_out())
        // .unwrap_or_else(|x| println!("routing errored: {}", x));

    // direct_route(in_device.midi_in(), out_device.midi_out())
        // .unwrap_or_else(|x| println!("routing errored: {}", x));

    in_device.close_port("Launchpad".to_string())
        .unwrap_or_else(|x| println!("closing errored: {}", x));
    out_device.close_port("Launchpad".to_string())
        .unwrap_or_else(|x| println!("closing errored: {}", x));
}
