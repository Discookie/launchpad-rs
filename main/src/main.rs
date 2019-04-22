use midichan_core::device::{MidiDevice, HasInput, HasOutput, Application};
use physical::{InputDevice, OutputDevice};
use launchpad::Launchpad;

use select::Select;
use simple::{DisplayPressed, DrawOneColor, Rainbow};


const IN_PORT:  usize = 0;
const OUT_PORT: usize = 1;


fn main() {
    let in_device = InputDevice::new();
    let out_device = OutputDevice::new();

    in_device.open_port("Launchpad".to_string(), IN_PORT)
        .unwrap_or_else(|x| println!("opening errored: {}", x));
    out_device.open_port("Launchpad".to_string(), OUT_PORT)
        .unwrap_or_else(|x| println!("opening errored: {}", x));

    let launchpad = Launchpad::new(in_device.midi_in(), out_device.midi_out());
        
    let dp = DisplayPressed::new(in_device.midi_in(), out_device.midi_out());
    let doc = DrawOneColor::new(in_device.midi_in(), out_device.midi_out());
    let rb = Rainbow::new(launchpad.clone());

    let mut select = Select::new(launchpad);
    select.add(Box::new(dp));
    select.add(Box::new(doc));
    select.add(Box::new(rb));

    select.run()
        .unwrap_or_else(|x| println!("running select errored: {}", x));

    in_device.close_port("Launchpad".to_string())
        .unwrap_or_else(|x| println!("closing errored: {}", x));
    out_device.close_port("Launchpad".to_string())
        .unwrap_or_else(|x| println!("closing errored: {}", x));
}
