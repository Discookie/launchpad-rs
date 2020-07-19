use crossbeam_channel::{Sender, Receiver};

macro_rules! num_to_enum {
    ($num:expr => $enm:ident{ $($fld:ident),+ }; $err:expr) => ({
        match $num {
            $(_ if $num == $enm::$fld as u8 => { $enm::$fld })+
            _ => $err
        }
    });
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum MessageType {
    SysEx = 0x70,
    NoteOff = 0x80,
    NoteOn = 0x90,
    NoteVelocity = 0xA0,
    CC = 0xB0,
    PC = 0xC0,
    CCVelocity = 0xD0,
    PitchBend = 0xE0,
    Unknown = 0xFE
}

impl MessageType {
    pub fn from_u8(num: u8) -> MessageType {
        num_to_enum!(
            num => MessageType{SysEx, NoteOff, NoteOn, NoteVelocity, CC, PC, CCVelocity, PitchBend};
            MessageType::Unknown
        )
    }
}

// #[derive(Clone)]
// pub enum SystemMessageType {
    // SysEx = 0xF0,
    // Timecode = 0xF1,
    // SongPosition = 0xF2,
    // SongSelect = 0xF3,
    // TuneRequest = 0xF6,
    // EndOfSysEx = 0xF7
// }

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct MidiMessage {
    pub device: String,
    pub timestamp: u64,
    pub channel: u8,
    pub msg_type: MessageType,
    pub key: u8,
    pub velocity: u8,
    pub sysex: Option<Vec<u8>>
}

impl MidiMessage {
    pub fn from_raw(name: &str, timestamp: u64, slice: &[u8]) -> MidiMessage {
        //println!("Msg get!: {:?}", slice);
        let msg_type = slice[0] & 0xF0;

        if matches!(msg_type, 0x70 | 0xD0) {
            let mut vec= Vec::new();
            vec.extend_from_slice(&slice[1..slice.len() - 1]);

            MidiMessage{
                device: name.to_string(),
                timestamp,
                channel: 0,
                msg_type: MessageType::from_u8(msg_type),
                key: 0,
                velocity: 0,
                sysex: Some(vec)
            }
        } else {
            let channel = slice[0] & 0x0F;
            let key = slice[1];
            let velocity = slice[2];
            MidiMessage{
                device: name.to_string(),
                timestamp,
                channel,
                msg_type: MessageType::from_u8(msg_type),
                key,
                velocity,
                sysex: None
            }
        }
    }

    pub fn new(name: &str) -> MidiMessage {
        MidiMessage{
            device: name.to_string(),
            timestamp: 0,
            channel: 0,
            msg_type: MessageType::NoteOn,
            key: 0,
            velocity: 0,
            sysex: None
        }
    }

    pub fn with_timestamp(&mut self, timestamp: u64) -> &mut MidiMessage {
        self.timestamp = timestamp;
        self
    }

    pub fn with_channel(&mut self, channel: u8) -> &mut MidiMessage {
        self.channel = channel;
        self
    }

    pub fn with_msg_type(&mut self, msg_type: MessageType) -> &mut MidiMessage {
        self.msg_type = msg_type;
        self
    }

    pub fn with_key(&mut self, key: u8) -> &mut MidiMessage {
        self.key = key;
        self
    }

    pub fn with_velocity(&mut self, velocity: u8) -> &mut MidiMessage {
        self.velocity = velocity;
        self
    }

    pub fn to_raw(&self) -> Vec<u8> {
        match self.msg_type {
            MessageType::SysEx => self.sysex.to_owned().unwrap_or_default(),
            _ => vec![(self.msg_type.clone() as u8) | self.channel, self.key, self.velocity]
        }
        
    }
}

#[derive(Clone)]
pub enum DeviceResponse {
    Device(String, bool),
    List(Vec<String>),
    Error(String),
    Ok
}

#[derive(Clone)]
pub enum DeviceRequest {
    OpenIn(String, usize),
    OpenOut(String, usize),

    QueryDevice(String),
    QueryList,

    CloseIn(String),
    CloseOut(String),

    Shutdown
}

#[derive(Clone)]
pub enum RouterResponse {
    Device(String, bool),

    List(Vec<String>),

    Error(String),
    Ok
}

#[derive(Clone)]
pub enum RouterRequest {
    AddInput(String, Receiver<MidiMessage>),
    RemoveInput(String),
    AddOutput(String, Sender<MidiMessage>),
    RemoveOutput(String),

    QueryInput(String),
    QueryOutput(String),
    QueryAllInputs,
    QueryAllOutputs,

    Shutdown
}
