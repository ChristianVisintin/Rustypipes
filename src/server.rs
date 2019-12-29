//! ## Server
//!
//! `server` is the module which takes care of managin the Octopipes Server struct and
//! then all the functions useful to implement an Octopipes Server

//
//   RustyPipes
//   Developed by Christian Visintin
//
// MIT License
// Copyright (c) 2019-2020 Christian Visintin
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
//

use super::OctopipesCapError;
use super::OctopipesCapMessage;
use super::OctopipesError;
use super::OctopipesMessage;
use super::OctopipesOptions;
use super::OctopipesProtocolVersion;
use super::OctopipesServer;
use super::OctopipesServerState;
use super::OctopipesServerWorker;
use super::OctopipesServerWorkerWrapper;
use super::Subscription;

use super::cap;
use super::pipes;
use super::serializer;

use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

impl OctopipesServer {
    /// ###  new
    ///
    /// `new` instances a new OctopipesServer
    pub fn new(version: OctopipesProtocolVersion, cap_pipe: String, on_recv_cb: fn(Result<&OctopipesMessage, &OctopipesError>)) -> OctopipesServer {
        OctopipesServer {
            version: version,
            state: Arc::new(Mutex::new(OctopipesServerState::Initialized)),
            cap_pipe: cap_pipe,
            cap_receiver: None,
            on_receive: on_recv_cb,
            cap_listener: None,
            workers: Vec::new()
        }
    }

    /// ###  start_cap_listener
    ///
    /// `start_cap_listener` Start CAP listener thread
    pub fn start_cap_listener(&mut self) -> Result<(), OctopipesError> {
        //Check if thread is already running
        if self.cap_listener.is_some() {
            return Err(OctopipesError::ThreadAlreadyRunning)
        }
        //Create CAP copy
        let cap_pipe: String = self.cap_pipe.clone();
        //Create CAP
        if let Err(_) = pipes::pipe_create(&cap_pipe) {
            return Err(OctopipesError::OpenFailed);
        }
        //Set server to running
        {
            let mut server_state = self.state.lock().unwrap();
            *server_state = OctopipesServerState::Running;
        }
        //Start thread
        let server_state_clone = Arc::clone(&self.state);
        let (cap_sender, cap_receiver) = mpsc::channel();
        self.cap_receiver = Some(cap_receiver);
        self.cap_listener = Some(thread::spawn(move || {
            loop {
                {
                    let current_server_state = server_state_clone.lock().unwrap();
                    //If state is not Runnning, exit
                    if *current_server_state != OctopipesServerState::Running {
                        break;
                    }
                }
                //Listen on CAP
                if let Ok(data_in) = pipes::pipe_read(&cap_pipe, 5000) {
                    if data_in.len() == 0 {
                        thread::sleep(Duration::from_millis(200));
                        continue;
                    }
                    //Parse message
                }
            }
        }));
        Ok(())
    }

    //TODO: drop
    //TODO: stop cap listener (which must deletes CAP too)
    //TODO: create worker
    //TODO: stop worker
}
