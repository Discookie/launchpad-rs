use std::error::Error;
use std::iter::once;

use crossbeam_channel::{Sender, Receiver};
use midichan_core::message::{MidiMessage, MessageType};

pub const BYTE_HEADER: [u8; 6] = [0xF0, 0x00, 0x20, 0x29, 0x02, 0x0C];


macro_rules! led_index {
    ($x: expr, $y: expr) => ( ($y+1) * 10 + ($x+1) )
}

#[macro_export]
macro_rules! lpx_color {
    ($x:expr) => { ::launchpad_x::Color { color: $x, pulse_mode: ::launchpad_x::PulseMode::Static }};
    ($x:expr, static) => { ::launchpad_x::Color { color: $x, pulse_mode: ::launchpad_x::PulseMode::Static }};
    ($x:expr, flash) => { ::launchpad_x::Color { color: $x, pulse_mode: ::launchpad_x::PulseMode::Flash }};
    ($x:expr, pulse) => { ::launchpad_x::Color { color: $x, pulse_mode: ::launchpad_x::PulseMode::Pulse }};
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum PulseMode {
    Static = 0x00,
    Flash = 0x01,
    Pulse = 0x02,
}

#[derive(Clone, Copy)]
pub struct Color {
    pub color: u8,
    pub pulse_mode: PulseMode
}

/// Sent out by the Sysex color setter message
/// Each color is 7-bit.
#[derive(Clone, Copy)]
pub struct LargeColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum LaunchpadScreen {
    Session = 0x00,
    Notes = 0x01,
    Custom1 = 0x04,
    Custom2 = 0x05,
    Custom3 = 0x06,
    Custom4 = 0x07,
    Faders = 0x0D,
    Programmer = 0x7F
}

#[derive(Clone)]
pub struct LaunchpadX {
    daw_name: String,
    midi_name: String,

    input: Receiver<MidiMessage>,
    output: Sender<MidiMessage>,
    daw_input: Receiver<MidiMessage>,
    daw_output: Sender<MidiMessage>,

    is_programmer_mode: bool,
    daw_mode: LaunchpadScreen
}

impl LaunchpadX {
    /// DAW in port is MIDIIN1, In port is MIDIIN2
    pub fn new(
        input: Receiver<MidiMessage>, output: Sender<MidiMessage>,
        daw_input: Receiver<MidiMessage>, daw_output: Sender<MidiMessage>
    ) -> Result<LaunchpadX, Box<dyn Error>> {
        let mut lp = LaunchpadX {
            daw_name: "Launchpad DAW".to_string(),
            midi_name: "Launchpad MIDI".to_string(),
            input, output, daw_input, daw_output,

            is_programmer_mode: true,
            daw_mode: LaunchpadScreen::Session
        };
        lp.send_sysex(&[0x10, 0x01])?;
        lp.set_programmer_mode(false)?;

        Ok(lp)
    }

    pub fn with_name(&mut self, daw_name: String, midi_name: String) -> &mut LaunchpadX {
        self.daw_name = daw_name;
        self.midi_name = midi_name;
        self
    }

    pub fn input(&self) -> Receiver<MidiMessage> {
        self.input.clone()
    }

    pub fn output(&self) -> Sender<MidiMessage> {
        self.output.clone()
    }

    pub fn daw_input(&self) -> Receiver<MidiMessage> {
        self.daw_input.clone()
    }

    pub fn daw_output(&self) -> Sender<MidiMessage> {
        self.daw_output.clone()
    }

    pub fn send_sysex(&self, sysex: &[u8]) -> Result<(), Box<dyn Error>> {
        Ok(self.output.send(
            MidiMessage{
                device: self.midi_name.clone(),
                timestamp: 0,
                channel: 0,
                msg_type: MessageType::SysEx,
                key: 0,
                velocity: 0,
                sysex: Some(
                    BYTE_HEADER.iter().copied()
                    .chain(sysex.iter().copied())
                    .chain(once(0xF7))
                    .collect()
                )
            }
        )?)
    }

    pub fn send_daw_sysex(&self, sysex: &[u8]) -> Result<(), Box<dyn Error>> {
        Ok(self.daw_output.send(
            MidiMessage{
                device: self.daw_name.clone(),
                timestamp: 0,
                channel: 0,
                msg_type: MessageType::SysEx,
                key: 0,
                velocity: 0,
                sysex: Some(
                    BYTE_HEADER.iter().copied()
                    .chain(sysex.iter().copied())
                    .chain(once(0xF7))
                    .collect()
                )
            }
        )?)
    }

    pub fn is_programmer_mode(&self) -> bool {
        self.is_programmer_mode
    }

    pub fn set_programmer_mode(&mut self, new_mode: bool) -> Result<(), Box<dyn Error>> {
        if self.is_programmer_mode != new_mode {
            self.send_sysex(&[
                0x0E,
                new_mode as u8
            ])?;

            self.is_programmer_mode = new_mode;
        }
        Ok(())
    }

    pub fn clear(&self) -> Result<(), Box<dyn Error>> {
        Ok(self.output.send(
            MidiMessage{
                device: self.midi_name.clone(),
                timestamp: 0,
                channel: 0,
                msg_type: MessageType::CC,
                key: 0,
                velocity: 0,
                sysex: None
            }
        )?)
    }

    pub fn set(&self, x: u8, y: u8, color: Color) -> Result<(), Box<dyn Error>> {
        Ok(self.output.send(
            MidiMessage{
                device: self.midi_name.clone(),
                timestamp: 0,
                channel: color.pulse_mode as u8,
                msg_type: match (x, y) {
                    (8, _) => MessageType::CC,
                    (_, 8) => MessageType::CC,
                    _ => MessageType::NoteOn
                },
                key: led_index!(x, y),
                velocity: color.color,
                sysex: None
            }
        )?)
    }

    /// Programmer mode only.
    pub fn set_large(&self, x: u8, y: u8, color: LargeColor) -> Result<(), Box<dyn Error>> {
        self.send_sysex(&[
            0x03,
            led_index!(x, y),
            color.red,
            color.green,
            color.blue
        ])?;

        Ok(())
    }

    pub fn set_screen(&mut self, screen: LaunchpadScreen) -> Result<(), Box<dyn Error>> {
        if !self.is_programmer_mode {
            self.send_sysex(&[
                0x00,
                screen as u8
            ])?;

            self.is_programmer_mode = matches!(screen, LaunchpadScreen::Programmer);
            self.daw_mode = screen;
        }

        Ok(())
    }

    pub fn set_session(&self, x: u8, y: u8, color: Color) -> Result<(), Box<dyn Error>> {
        Ok(self.daw_output.send(
            MidiMessage{
                device: self.daw_name.clone(),
                timestamp: 0,
                channel: color.pulse_mode as u8,
                msg_type: match (x, y) {
                    (8, _) => MessageType::CC,
                    (_, 8) => MessageType::CC,
                    _ => MessageType::NoteOn
                },
                key: led_index!(x, y),
                velocity: color.color,
                sysex: None
            }
        )?)
    }

    /// Only set the faders you want to change.
    ///
    /// Fader values:
    ///  - True on bi-polar entries
    ///  - CC value set by fader
    ///  - Fader color
    pub fn init_faders(&self, is_horizontal: bool, faders: &[Option<(bool, u8, Color)>; 8]) -> Result<(), Box<dyn Error>> {
        let mut concat_sysex: Vec<u8> = 
            once(0x01)
            .chain(once(0x00))
            .chain(once(is_horizontal as u8))
            .collect();

        for f in faders {
            match f {
                Some((is_bipolar, cc, color)) => concat_sysex.extend_from_slice(&[
                    *is_bipolar as u8,
                    *cc,
                    color.color
                ]),
                _ => ()
            };
        }

        self.send_daw_sysex(concat_sysex.as_slice())?;

        Ok(())
    }

    pub fn set_fader_pos(&self, fader: u8, pos: u8) -> Result<(), Box<dyn Error>> {
        Ok(self.daw_output.send(
            MidiMessage{
                device: self.daw_name.clone(),
                timestamp: 0,
                channel: 4,
                msg_type: MessageType::CC,
                key: fader,
                velocity: pos,
                sysex: None
            }
        )?)
    }

    pub fn set_fader_color(&self, fader: u8, color: Color) -> Result<(), Box<dyn Error>> {
        Ok(self.daw_output.send(
            MidiMessage{
                device: self.daw_name.clone(),
                timestamp: 0,
                channel: 5,
                msg_type: MessageType::CC,
                key: fader,
                velocity: color.color,
                sysex: None
            }
        )?)
    }

    /// 0: Disable  
    // 1: Simple mode, not scrollable  
    // 2: Intelligent mode, scrollable
    pub fn set_drum_rack_mode(&self, drum_rack_mode: u8) -> Result<(), Box<dyn Error>> {
        self.send_daw_sysex(&[
            0x0F,
            drum_rack_mode
        ])?;

        Ok(())
    }

    pub fn set_drum_rack(&self, x: u8, y: u8, color: Color) -> Result<(), Box<dyn Error>> {
        Ok(self.daw_output.send(
            MidiMessage{
                device: self.daw_name.clone(),
                timestamp: 0,
                channel: 8 + color.pulse_mode as u8,
                msg_type: match (x, y) {
                    (8, _) => MessageType::CC,
                    (_, 8) => MessageType::CC,
                    _ => MessageType::NoteOn
                },
                key: led_index!(x, y),
                velocity: color.color,
                sysex: None
            }
        )?)
    }

    pub fn clear_daw_state(&self, clear_session: bool, clear_drum_rack: bool, clear_cc: bool) -> Result<(), Box<dyn Error>> {
        self.send_daw_sysex(&[
            0x12,
            clear_session as u8,
            clear_drum_rack as u8,
            clear_cc as u8
        ])
    }

    pub fn scroll_text(&self, text: &str, color: Color, speed: u8, is_loop: bool) -> Result<(), Box<dyn Error>> {
        let mut text_sysex = vec![
            0x07,
            is_loop as u8,
            speed,
            0,
            color.color
        ];
        text_sysex.extend_from_slice(text.as_bytes());
        text_sysex.retain(|x| *x != 0xF7);

        self.send_daw_sysex(text_sysex.as_slice())?;

        Ok(())
    }

    pub fn scroll_text_large(&self, text: &str, color: LargeColor, speed: u8, is_loop: bool) -> Result<(), Box<dyn Error>> {
        let mut text_sysex = vec![
            0x07,
            is_loop as u8,
            speed,
            1,
            color.red,
            color.green,
            color.blue
        ];

        text_sysex.extend_from_slice(text.as_bytes());
        text_sysex.retain(|x| *x != 0xF7);

        self.send_daw_sysex(text_sysex.as_slice())?;

        Ok(())
    }

    pub fn stop_scroll_text(&self) -> Result<(), Box<dyn Error>> {
        self.send_daw_sysex(&[
            0x07
        ])?;

        Ok(())
    }

    pub fn set_sleep(&self, should_sleep: bool) -> Result<(), Box<dyn Error>> {
        self.send_daw_sysex(&[
            0x09,
            should_sleep as u8
        ])
    }
}

impl Drop for LaunchpadX {
    fn drop(&mut self) {
        self.set_programmer_mode(false).ok();
        self.set_screen(LaunchpadScreen::Custom3).ok();
        self.clear_daw_state(true, true, true).ok();
        self.send_sysex(&[0x10, 0x00]).ok();
    }
}