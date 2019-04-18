use crate::messages::MidiMessage;
use crossbeam_channel::{Sender, Receiver};

pub trait Controllable<R, T> {
    fn control_request(&self) -> Sender<R>;
    fn control_response(&self) -> Receiver<T>;
}

pub trait MidiDevice {
    fn open_port(&self, name: String, port_id: usize) -> Result<(), String>;
    fn close_port(&self, name: String) -> Result<(), String>;

    fn query(&self, name: String) -> Result<bool, String>;
    fn query_all(&self) -> Result<Vec<String>, String>;
}

pub trait RoutingDevice {
    fn add_input(&self, name: String, port: Receiver<MidiMessage>) -> Result<(), String>;
    fn add_output(&self, name: String, port: Sender<MidiMessage>) -> Result<(), String>;

    fn remove_input(&self, name: String) -> Result<(), String>;
    fn remove_output(&self, name: String) -> Result<(), String>;
    
    fn query_input(&self, name: String) -> Result<bool, String>;
    fn query_output(&self, name: String) -> Result<bool, String>;
    fn query_all_inputs(&self) -> Result<Vec<String>, String>;
    fn query_all_outputs(&self) -> Result<Vec<String>, String>;
    fn query_all(&self) -> Result<(Vec<String>, Vec<String>), String>;
}