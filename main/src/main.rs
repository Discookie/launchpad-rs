use midichan_core::device::{MidiDevice, HasInput, HasOutput, Application};
use physical::{InputDevice, OutputDevice};


/// Uses the launchpad Mini.
#[cfg(feature = "mini")]
pub fn main() {
    use launchpad::{Launchpad, Color};
    
    use select::Select;
    use simple::{DisplayPressed, DrawOneColor, Rainbow};
    use chain_reaction::ChainReaction;

    const IN_PORT:  usize = 2;
    const OUT_PORT: usize = 2;


    let in_device = InputDevice::new();
    let out_device = OutputDevice::new();

    in_device.open_port("Launchpad".to_string(), IN_PORT)
        .unwrap_or_else(|x| println!("opening errored: {}", x));
    out_device.open_port("Launchpad".to_string(), OUT_PORT)
        .unwrap_or_else(|x| println!("opening errored: {}", x));

    let launchpad = Launchpad::new(in_device.midi_in(), out_device.midi_out());
        
    let mut select = {
        let dp = DisplayPressed::new(in_device.midi_in(), out_device.midi_out());
        let mut doc = DrawOneColor::new(in_device.midi_in(), out_device.midi_out());
        doc.with_color(&Color::new(3, 3));
        let rb = Rainbow::new(launchpad.clone());
        let chain = ChainReaction::new(launchpad.clone());
        
        let mut select = Select::new(launchpad);
        select.add(Box::new(dp));
        select.add(Box::new(doc));
        select.add(Box::new(rb));
        select.add(Box::new(chain));

        select
    };

    select.run()
        .unwrap_or_else(|x| println!("running select errored: {}", x));

    in_device.close_port("Launchpad".to_string())
        .unwrap_or_else(|x| println!("closing errored: {}", x));
    out_device.close_port("Launchpad".to_string())
        .unwrap_or_else(|x| println!("closing errored: {}", x));
}

/// Uses the Launchpad X.
#[cfg(feature = "x")]
pub fn main() {
    use launchpad_x::{LaunchpadX, Color};

    use select_x::Select;
    use simple_x::{DisplayPressed, DrawOneColor, Rainbow};
    use chain_reaction_x::ChainReaction;

    const DAW_IN:  usize = 7;
    const DAW_OUT:  usize = 7;
    const MIDI_IN:  usize = 8;
    const MIDI_OUT:  usize = 8;

    let daw_in = InputDevice::new();
    let daw_out = OutputDevice::new();
    let midi_in = InputDevice::new();
    let midi_out = OutputDevice::new();

    daw_in.open_port("Launchpad DAW".to_string(), DAW_IN)
        .unwrap_or_else(|x| println!("opening errored: {}", x));
    daw_out.open_port("Launchpad DAW".to_string(), DAW_OUT)
        .unwrap_or_else(|x| println!("opening errored: {}", x));
    midi_in.open_port("Launchpad MIDI".to_string(), MIDI_IN)
        .unwrap_or_else(|x| println!("opening errored: {}", x));
    midi_out.open_port("Launchpad MIDI".to_string(), MIDI_OUT)
        .unwrap_or_else(|x| println!("opening errored: {}", x));

    let launchpad = LaunchpadX::new(
        midi_in.midi_in(), midi_out.midi_out(),
        daw_in.midi_in(), daw_out.midi_out(),
    ).expect("Failed to open Launchpad X in DAW mode");
        
    let mut select = {
        let dp = DisplayPressed::new(launchpad.clone());
        let doc = DrawOneColor::new(launchpad.clone());
        let rb = Rainbow::new(launchpad.clone());
        let cr = ChainReaction::new(launchpad.clone());
        // let chain = ChainReaction::new(launchpad.clone());
        
        let mut select = Select::new(launchpad);
        select.add("Display pressed".to_string(), Box::new(dp));
        select.add("Draw one color".to_string(), Box::new(doc));
        select.add("Rainbow".to_string(), Box::new(rb));
        select.add("Chain Reaction".to_string(), Box::new(cr));
        //select.add(Box::new(chain));

        select
    };

    select.run()
        .unwrap_or_else(|x| println!("running select errored: {}", x));

    drop(select);

    daw_in.close_port("Launchpad DAW".to_string())
        .unwrap_or_else(|x| println!("closing errored: {}", x));
    daw_out.close_port("Launchpad DAW".to_string())
        .unwrap_or_else(|x| println!("closing errored: {}", x));
    midi_in.close_port("Launchpad MIDI".to_string())
        .unwrap_or_else(|x| println!("closing errored: {}", x));
    midi_out.close_port("Launchpad MIDI".to_string())
        .unwrap_or_else(|x| println!("closing errored: {}", x));
}
