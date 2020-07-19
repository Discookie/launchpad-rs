#[macro_use]
mod macros;

use std::time::Duration;
use std::error::Error;
use std::thread;

use hashbrown::HashMap;
use crossbeam_channel::{bounded, Sender, Receiver, Select};

use midichan_core::message::{RouterRequest, RouterResponse, MidiMessage};
use midichan_core::device::{Controllable, RoutingDevice};

const TIMEOUT: Duration = Duration::from_secs(1);

pub struct Router {
    control_request: Sender<RouterRequest>,
    control_response: Receiver<RouterResponse>
}

impl Router {
    pub fn with_function  <T: 'static + Send + Copy + Fn(&mut MidiMessage) -> Vec<String>>
                        (router_func: T) -> Router {
        let (exported_send, thread_recv) = bounded(0);
        let (thread_send, exported_recv) = bounded(2);

        let cloned_func = router_func;

        thread::spawn(move || {
            router_wrapper(cloned_func, thread_recv, thread_send);
        });

        Router{control_request: exported_send, control_response: exported_recv}
    }

    pub fn split_by_device() -> Router {
        fn by_device_func(dev: &mut MidiMessage) -> Vec<String> {
            vec!(dev.device.clone())
        }

        Router::with_function(by_device_func)
    }

    pub fn on_off() -> Router {
        fn on_off_func(dev: &mut MidiMessage) -> Vec<String> {
            vec!(match dev.velocity {
                0 => "off",
                _ => "on"
            }.to_string())
        }

        Router::with_function(on_off_func)
    }

    pub fn mirror_all() -> Router {
        fn all_func(_dev: &mut MidiMessage) -> Vec<String> {
            vec!("all".to_string())
        }
        
        Router::with_function(all_func)
    }
}

impl Controllable<RouterRequest, RouterResponse> for Router {
    fn control_request(&self) -> Sender<RouterRequest> {
        self.control_request.clone()
    }

    fn control_response(&self) -> Receiver<RouterResponse> {
        self.control_response.clone()
    }
}

impl RoutingDevice for Router {
    fn add_input(&self, name: String, port: Receiver<MidiMessage>) -> Result<(), String> {
        error_on_full!(self.control_response, "router");
        send_or_err!(self.control_request, RouterRequest::AddInput(name, port), "router");

        match self.control_response.recv_timeout(TIMEOUT) {
            Ok(RouterResponse::Ok) => Ok(()),
            Ok(RouterResponse::Error(err)) => Err(err.to_string()),

            Ok(_) => Err("router desync".to_string()),
            Err(_) => Err("router timed out".to_string())
        }
    }

    fn add_output(&self, name: String, port: Sender<MidiMessage>) -> Result<(), String> {
        error_on_full!(self.control_response, "router");
        send_or_err!(self.control_request, RouterRequest::AddOutput(name, port), "router");

        match self.control_response.recv_timeout(TIMEOUT) {
            Ok(RouterResponse::Ok) => Ok(()),
            Ok(RouterResponse::Error(err)) => Err(err.to_string()),

            Ok(_) => Err("router desync".to_string()),
            Err(_) => Err("router timed out".to_string())
        }
    }

    fn remove_input(&self, name: String) -> Result<(), String> {
        error_on_full!(self.control_response, "router");
        send_or_err!(self.control_request, RouterRequest::RemoveInput(name), "router");

        match self.control_response.recv_timeout(TIMEOUT) {
            Ok(RouterResponse::Ok) => Ok(()),
            Ok(RouterResponse::Error(err)) => Err(err.to_string()),

            Ok(_) => Err("router desync".to_string()),
            Err(_) => Err("router timed out".to_string())
        }
    }

    fn remove_output(&self, name: String) -> Result<(), String> {
        error_on_full!(self.control_response, "router");
        send_or_err!(self.control_request, RouterRequest::RemoveOutput(name), "router");

        match self.control_response.recv_timeout(TIMEOUT) {
            Ok(RouterResponse::Ok) => Ok(()),
            Ok(RouterResponse::Error(err)) => Err(err.to_string()),

            Ok(_) => Err("router desync".to_string()),
            Err(_) => Err("router timed out".to_string())
        }
    }

    fn query_input(&self, name: String) -> Result<bool, String>{
        error_on_full!(self.control_response, "router");
        send_or_err!(self.control_request, RouterRequest::QueryInput(name), "router");
        // self.control_request.send(
            // RouterRequest::QueryDevice(name) );

        match self.control_response.recv_timeout(TIMEOUT) {
            Ok(RouterResponse::Device(_, status))  => Ok(status),
            Ok(RouterResponse::Error(err)) => Err(err.to_string()),

            Ok(_) => Err("router desync".to_string()),
            Err(_) => Err("router timed out".to_string())
        }
    }
    
    fn query_output(&self, name: String) -> Result<bool, String>{
        error_on_full!(self.control_response, "router");
        send_or_err!(self.control_request, RouterRequest::QueryOutput(name), "router");
        // self.control_request.send(
            // RouterRequest::QueryDevice(name) );

        match self.control_response.recv_timeout(TIMEOUT) {
            Ok(RouterResponse::Device(_, status))  => Ok(status),
            Ok(RouterResponse::Error(err)) => Err(err.to_string()),

            Ok(_) => Err("router desync".to_string()),
            Err(_) => Err("router timed out".to_string())
        }
    }

    fn query_all_inputs(&self) -> Result<Vec<String>, String> {
        error_on_full!(self.control_response, "router");
        send_or_err!(self.control_request, RouterRequest::QueryAllInputs, "router");

        match self.control_response.recv_timeout(TIMEOUT) {
            Ok(RouterResponse::List(list)) => Ok(list),
            Ok(RouterResponse::Error(err)) => Err(err.to_string()),

            Ok(_) => Err("router desync".to_string()),
            Err(_) => Err("router timed out".to_string())
        }
    }

    fn query_all_outputs(&self) -> Result<Vec<String>, String> {
        error_on_full!(self.control_response, "router");
        send_or_err!(self.control_request, RouterRequest::QueryAllOutputs, "router");

        match self.control_response.recv_timeout(TIMEOUT) {
            Ok(RouterResponse::List(list)) => Ok(list),
            Ok(RouterResponse::Error(err)) => Err(err.to_string()),

            Ok(_) => Err("router desync".to_string()),
            Err(_) => Err("router timed out".to_string())
        }
    }

    fn query_all(&self) -> Result<(Vec<String>, Vec<String>), String> {
        let inputs = self.query_all_inputs()?;
        let outputs = self.query_all_outputs()?;

        Ok((inputs, outputs))
    }
}

impl Drop for Router {
    fn drop(&mut self) {
        self.control_request.send(
            RouterRequest::Shutdown ).unwrap();
        self.control_response.recv_timeout(TIMEOUT).unwrap();
    }
}


fn router_wrapper  <T: 'static + Send + Copy + Fn(&mut MidiMessage) -> Vec<String>>
                (router_func: T, control_request: Receiver<RouterRequest>, control_response: Sender<RouterResponse>) {
    match router_thread(router_func, &control_request, &control_response) {
        Ok(()) => {},

        Err(err) => { //control_response.send( 
            // RouterResponse::Error(format!("router died: {}", err.to_string()) )); }
            panic!("router died: {}", err.to_string());
        }
    };
}

fn router_thread   <T: 'static + Send + Copy + Fn(&mut MidiMessage) -> Vec<String>>
                (router_func: T, control_request: &Receiver<RouterRequest>, control_response: &Sender<RouterResponse>) -> Result<(), Box<dyn Error>> {
    let mut in_map = HashMap::new();
    let mut out_map = HashMap::new();

    let mut select_map = HashMap::new();

    loop {
        let (result, control_id) = {
            let mut select = Select::new();

            for (name, input) in in_map.iter() {
                let _refname: &String = name;
                let cloned_name: String = name.clone();
                select_map.insert(select.recv(input), cloned_name);
            }

            let control_id = select.recv(control_request);

            let res = select.select();

            (res, control_id)
        };

        let index = result.index();
        
        if index == control_id {
            match result.recv(control_request)? {
                RouterRequest::AddInput(name, port) => {
                    in_map.insert(name, port);
                    
                    control_response.send(
                        RouterResponse::Ok)?;
                },

                RouterRequest::AddOutput(name, port) => {
                    out_map.insert(name, port);
                    
                    control_response.send(
                        RouterResponse::Ok)?;
                },

                RouterRequest::RemoveInput(name) => {
                    in_map.remove(&name);

                    control_response.send(
                        RouterResponse::Ok)?;
                },

                RouterRequest::RemoveOutput(name) => {
                    out_map.remove(&name);

                    control_response.send(
                        RouterResponse::Ok)?;
                },

                RouterRequest::QueryInput(name) => {
                    let cont = in_map.contains_key(&name);
                    control_response.send(
                        RouterResponse::Device(name, cont))?;
                },
                
                RouterRequest::QueryOutput(name) => {
                    let cont = out_map.contains_key(&name);
                    control_response.send(
                        RouterResponse::Device(name, cont))?;
                },

                RouterRequest::QueryAllInputs => {
                    control_response.send(
                        RouterResponse::List(in_map.keys().cloned().collect()))?;
                },

                RouterRequest::QueryAllOutputs => {
                    control_response.send(
                        RouterResponse::List(in_map.keys().cloned().collect()))?;
                },

                RouterRequest::Shutdown => {
                    control_response.send(
                        RouterResponse::Ok)?;
                    return Ok(());
                },
            }
        } else {
            let mut msg = match result.recv(&in_map[&select_map[&index]]) {
                Ok(msg) => msg,
                Err(_) => continue
            };

            let targets = router_func(&mut msg);

            for target in targets {
                if target == "all" {
                    for output in out_map.values() {
                        output.send(msg.clone()).ok();
                    }
                } else {
                    out_map.get(&target).map(|x| x.send(msg.clone()));
                }
            }
        }
    }
}
