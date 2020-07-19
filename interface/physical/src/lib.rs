#[macro_use]
extern crate crossbeam_channel;

extern crate midichan_core;

#[macro_use]
mod macros;

use std::time::Duration;
use std::error::Error;
use std::thread;

use hashbrown::HashMap;
use crossbeam_channel::{bounded, Sender, Receiver};
use midir::{MidiInput, MidiOutput, Ignore};

use midichan_core::message::{DeviceRequest, DeviceResponse, MidiMessage};
use midichan_core::device::{Controllable, HasInput, HasOutput, MidiDevice};

const TIMEOUT: Duration = Duration::from_secs(1);


pub struct InputDevice {
    control_request: Sender<DeviceRequest>,
    control_response: Receiver<DeviceResponse>,
    midi_in: Receiver<MidiMessage>
}

impl InputDevice {
    pub fn new() -> InputDevice {
        let (exported_send, thread_recv) = bounded(1);
        let (thread_send, exported_recv) = bounded(2);
        let (thread_midi, exported_midi) = bounded(128);

        thread::spawn(move || {
            input_wrapper(thread_midi, thread_recv, thread_send);
        });

        InputDevice{control_request: exported_send, control_response: exported_recv, midi_in: exported_midi}
    }
}

impl HasInput for InputDevice {
    fn midi_in(&self) -> Receiver<MidiMessage> {
        self.midi_in.clone()
    }
}

impl Controllable<DeviceRequest, DeviceResponse> for InputDevice {
    fn control_request(&self) -> Sender<DeviceRequest> {
        self.control_request.clone()
    }

    fn control_response(&self) -> Receiver<DeviceResponse> {
        self.control_response.clone()
    }
}

impl MidiDevice for InputDevice {
    fn open_port(&self, name: String, port_id: usize) -> Result<(), String> {
        error_on_full!(self.control_response, "input device");
        send_or_err!(self.control_request, DeviceRequest::OpenIn(name, port_id), "input device");

        match self.control_response.recv_timeout(TIMEOUT) {
            Ok(DeviceResponse::Ok) => Ok(()),
            Ok(DeviceResponse::Error(err)) => Err(err.to_string()),

            Ok(_) => Err("input device desync".to_string()),
            Err(_) => Err("input device timed out".to_string())
        }
    }

    fn close_port(&self, name: String) -> Result<(), String> {
        error_on_full!(self.control_response, "input device");
        send_or_err!(self.control_request, DeviceRequest::CloseIn(name), "input device");

        match self.control_response.recv_timeout(TIMEOUT) {
            Ok(DeviceResponse::Ok) => Ok(()),
            Ok(DeviceResponse::Error(err)) => Err(err.to_string()),

            Ok(_) => Err("input device desync".to_string()),
            Err(_) => Err("input device timed out".to_string())
        }
    }

    fn query(&self, name: String) -> Result<bool, String> {
        error_on_full!(self.control_response, "input device");
        send_or_err!(self.control_request, DeviceRequest::QueryDevice(name), "input device");
        // self.control_request.send(
            // DeviceRequest::QueryDevice(name) );

        match self.control_response.recv_timeout(TIMEOUT) {
            Ok(DeviceResponse::Device(_, status))  => Ok(status),
            Ok(DeviceResponse::Error(err)) => Err(err.to_string()),

            Ok(_) => Err("input device desync".to_string()),
            Err(_) => Err("input device timed out".to_string())
        }
    }

    fn query_all(&self) -> Result<Vec<String>, String> {
        error_on_full!(self.control_response, "input device");
        send_or_err!(self.control_request, DeviceRequest::QueryList, "input device");

        match self.control_response.recv_timeout(TIMEOUT) {
            Ok(DeviceResponse::List(list)) => Ok(list),
            Ok(DeviceResponse::Error(err)) => Err(err.to_string()),

            Ok(_) => Err("input device desync".to_string()),
            Err(_) => Err("input device timed out".to_string())
        }
    }
}

impl Drop for InputDevice {
    fn drop(&mut self) {
        self.control_request.send(
            DeviceRequest::Shutdown ).ok();
        self.control_response.recv_timeout(TIMEOUT).ok();
    }
}


fn input_wrapper(midi_out: Sender<MidiMessage>, control_request: Receiver<DeviceRequest>, control_response: Sender<DeviceResponse>) {
    match input_thread(midi_out, &control_request, &control_response) {
        Ok(()) => {},

        Err(err) => { //control_response.send( 
            // DeviceResponse::Error(format!("input handler died: {}", err.to_string()) )); }
            panic!("input handler died: {}", err.to_string());
        }
    };
}

fn input_thread(midi_out: Sender<MidiMessage>, control_request: &Receiver<DeviceRequest>, control_response: &Sender<DeviceResponse>) -> Result<(), Box<dyn Error>> {
    let mut map = HashMap::new();

    loop {
        select!{
            recv(control_request) -> msg => match msg? {
                DeviceRequest::OpenIn(name, port) => {
                    if map.contains_key(&name) {
                        control_response.send(
                            DeviceResponse::Error("input handler: device already added".to_string()))?;
                        continue;
                    }

                    let clone_out = midi_out.clone();
                    let clone_response = control_response.clone();
                    let clone_name = name.clone();
                    
                    let mut in_bridge = MidiInput::new("launchpad-rs")?;
                    in_bridge.ignore(Ignore::All);

                    match in_bridge.connect(port, "launchpad-rs", 
                        move |timestamp: u64, message: &[u8], _: &mut _| {
                            clone_out.send_timeout(MidiMessage::from_raw(&clone_name, timestamp, message), TIMEOUT)
                                .or_else(|_| 
                                    clone_response.send(DeviceResponse::Error("input handler: channel timeout".to_string()) ) )
                                .expect("Could not send Error message");
                        }, ())
                    {
                        Ok(midi_in) => {
                            map.insert(name, midi_in);

                            control_response.send(
                                DeviceResponse::Ok)?;
                        },

                        Err(err) => {
                            control_response.send(
                                DeviceResponse::Error( format!("input handler: failed to add device: {}", err.to_string()) ))?
                        }
                    }

                },

                DeviceRequest::CloseIn(name) => {
                    map.remove(&name).map(|x| x.close());

                    control_response.send(
                        DeviceResponse::Ok)?;
                },

                DeviceRequest::QueryDevice(name) => {
                    control_response.send(
                        DeviceResponse::Device(name.clone(), map.contains_key(&name)))?;
                },

                DeviceRequest::QueryList => {
                    let mut in_bridge = MidiInput::new("launchpad-rs")?;
                    in_bridge.ignore(Ignore::All);

                    let response = (0..in_bridge.port_count())
                        .map( |x| in_bridge.port_name(x)
                                .unwrap_or("Error".to_string()))
                        .collect();

                    control_response.send(
                        DeviceResponse::List(response))?;
                },

                DeviceRequest::Shutdown => {
                    let keys: Vec<String> = map.keys().cloned().collect();

                    for key in keys {
                        map.remove(&key).map(|x| x.close());
                    }

                    control_response.send(
                        DeviceResponse::Ok)?;

                    return Ok(());
                },

                _ => {
                    control_response.send(
                        DeviceResponse::Error("output handler: unknown command".to_string()))?;
                }
            }
        }
    }
}



pub struct OutputDevice {
    control_request: Sender<DeviceRequest>,
    control_response: Receiver<DeviceResponse>,
    midi_out: Sender<MidiMessage>
}

impl OutputDevice {
    pub fn new() -> OutputDevice {
        let (exported_send, thread_recv) = bounded(0);
        let (thread_send, exported_recv) = bounded(2);
        let (exported_midi, thread_midi) = bounded(128);

        thread::spawn(move || {
            output_wrapper(thread_midi, thread_recv, thread_send);
        });

        OutputDevice{control_request: exported_send, control_response: exported_recv, midi_out: exported_midi}
    }
}

impl HasOutput for OutputDevice {
    fn midi_out(&self) -> Sender<MidiMessage> {
        self.midi_out.clone()
    }
}

impl Controllable<DeviceRequest, DeviceResponse> for OutputDevice {
    fn control_request(&self) -> Sender<DeviceRequest> {
        self.control_request.clone()
    }

    fn control_response(&self) -> Receiver<DeviceResponse> {
        self.control_response.clone()
    }
}

impl MidiDevice for OutputDevice {
    fn open_port(&self, name: String, port_id: usize) -> Result<(), String> {
        error_on_full!(self.control_response, "output device");
        send_or_err!(self.control_request, DeviceRequest::OpenOut(name, port_id), "output device");

        match self.control_response.recv_timeout(TIMEOUT) {
            Ok(DeviceResponse::Ok) => Ok(()),
            Ok(DeviceResponse::Error(err)) => Err(err.to_string()),

            Ok(_) => Err("output device desync".to_string()),
            Err(_) => Err("output device timed out".to_string())
        }
    }

    fn close_port(&self, name: String) -> Result<(), String> {
        error_on_full!(self.control_response, "output device");
        send_or_err!(self.control_request, DeviceRequest::CloseOut(name), "output device");

        match self.control_response.recv_timeout(TIMEOUT) {
            Ok(DeviceResponse::Ok) => Ok(()),
            Ok(DeviceResponse::Error(err)) => Err(err.to_string()),

            Ok(_) => Err("output device desync".to_string()),
            Err(_) => Err("output device timed out".to_string())
        }
    }

    fn query(&self, name: String) -> Result<bool, String> {
        error_on_full!(self.control_response, "output device");
        send_or_err!(self.control_request, DeviceRequest::QueryDevice(name), "output device");

        match self.control_response.recv_timeout(TIMEOUT) {
            Ok(DeviceResponse::Device(_, status))  => Ok(status),
            Ok(DeviceResponse::Error(err)) => Err(err.to_string()),

            Ok(_) => Err("output device desync".to_string()),
            Err(_) => Err("output device timed out".to_string())
        }
    }

    fn query_all(&self) -> Result<Vec<String>, String> {
        error_on_full!(self.control_response, "output device");
        send_or_err!(self.control_request, DeviceRequest::QueryList, "output device");

        match self.control_response.recv_timeout(TIMEOUT) {
            Ok(DeviceResponse::List(list)) => Ok(list),
            Ok(DeviceResponse::Error(err)) => Err(err.to_string()),

            Ok(_) => Err("output device desync".to_string()),
            Err(_) => Err("output device timed out".to_string())
        }
    }
}

impl Drop for OutputDevice {
    fn drop(&mut self) {
        self.control_request.send(
            DeviceRequest::Shutdown ).ok();
        self.control_response.recv_timeout(TIMEOUT).ok();
    }
}


fn output_wrapper(midi_in: Receiver<MidiMessage>, control_request: Receiver<DeviceRequest>, control_response: Sender<DeviceResponse>) {
    match output_thread(midi_in, &control_request, &control_response) {
        Ok(()) => {},
        Err(err) => { //control_response.send( 
            // DeviceResponse::Error(format!("output handler died: {}", err.to_string()) )); }
            panic!("output handler died: {}", err.to_string());
        }
    };
}

fn output_thread(midi_in: Receiver<MidiMessage>, control_request: &Receiver<DeviceRequest>, control_response: &Sender<DeviceResponse>) -> Result<(), Box<dyn Error>> {
    let mut map = HashMap::new();

    loop {
        select!{
            recv(control_request) -> msg => match msg? {
                DeviceRequest::OpenOut(name, port) => {
                    if map.contains_key(&name) {
                        control_response.send(
                            DeviceResponse::Error("output handler: device already added".to_string()))?;
                        continue;
                    }

                    let out_bridge = MidiOutput::new("launchpad-rs")?;

                    match out_bridge.connect(port, "launchpad-rs") {
                        Ok(midi_out) => {
                            map.insert(name, midi_out);

                            control_response.send(
                                DeviceResponse::Ok)?;
                        }

                        Err(err) => {
                            control_response.send(
                                DeviceResponse::Error( format!("output handler: failed to add device: {}", err.to_string()) ))?
                        }
                    }

                },

                DeviceRequest::CloseOut(name) => {
                    map.remove(&name).map(|x| x.close());

                    control_response.send(
                        DeviceResponse::Ok)?;
                }

                DeviceRequest::QueryDevice(name) => {
                    control_response.send(
                        DeviceResponse::Device(name.clone(), map.contains_key(&name)))?;
                },

                DeviceRequest::QueryList => {
                    let out_bridge = MidiOutput::new("launchpad-rs")?;

                    let response = (0..out_bridge.port_count())
                        .map( |x| out_bridge.port_name(x)
                                .unwrap_or("Error".to_string()))
                        .collect();

                    control_response.send(
                        DeviceResponse::List(response))?;
                },
                
                DeviceRequest::Shutdown => {
                    let keys: Vec<String> = map.keys().cloned().collect();

                    for key in keys {
                        map.remove(&key).map(|x| x.close());
                    }

                    control_response.send(
                        DeviceResponse::Ok)?;

                    return Ok(());
                },

                _ => {
                    control_response.send(
                        DeviceResponse::Error("output handler: unknown command".to_string()))?;
                }
            },
            
            recv(midi_in) -> msg => {
                let midi_msg = msg?;
                if let Some(midi_out) = map.get_mut(&midi_msg.device) {

                    if let Err(err) = midi_out.send(&midi_msg.to_raw()) {
                        control_response.send(
                            DeviceResponse::Error( format!("output handler: failed to add device: {}", err.to_string()) ))?
                    } else {
                        //println!("Sent! ({:?})", midi_msg.to_raw());
                    }
                }
            },
        }
    }
}