//! ## Client
//!
//! `client` is the module which takes care of managin the OctopipesClient struct and
//! then all the functions useful for the user to interface with an Octopipes Server

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

use super::Client;
use super::OctopipesCapError;
use super::OctopipesCapMessage;
use super::OctopipesClient;
use super::OctopipesError;
use super::OctopipesMessage;
use super::OctopipesOptions;
use super::OctopipesProtocolVersion;
use super::OctopipesState;

use std::sync::{Arc, Mutex};
use std::thread;

use super::cap;
use super::pipes;
use super::serializer;

impl OctopipesClient {
    /// ### OctopipesClient Constructor
    ///
    /// `new` is constructor for OctopipesClient. OctopipesClient is actually a wrapper for a Client
    pub fn new(
        client_id: String,
        cap_pipe: String,
        version: OctopipesProtocolVersion,
    ) -> OctopipesClient {
        OctopipesClient {
            this: Arc::new(Mutex::new(Client::new(client_id, cap_pipe, version))),
        }
    }

    //Thread operations

    /// ###  loop_start
    ///
    /// `loop_start` starts the client loop thread which checks if new messages are available
    pub fn loop_start(&mut self) -> Result<(), OctopipesError> {
        let mut client = self.this.lock().unwrap();
        match client.state {
            OctopipesState::Subscribed => {
                //Create threaded client
                let this_rc = Arc::clone(&self.this);
                client.client_loop = Some(thread::spawn(move || {
                    loop {
                        let client = this_rc.lock().unwrap();
                        if client.state != OctopipesState::Running {
                            break;
                        }
                        //Try to read (Read for 200 ms and sleep for 100ms)
                        match pipes::pipe_read(&client.rx_pipe.as_ref().unwrap(), 200) {
                            Ok(data) => {
                                if data.len() == 0 {
                                    drop(client);
                                    thread::sleep(std::time::Duration::from_millis(100));
                                    continue; //Just go on
                                }
                                //Otherwise parse message and send to callback
                                match serializer::decode_message(data) {
                                    Ok(message) => {
                                        //If message has ACK, send ACK back
                                        if message.options.intersects(OctopipesOptions::RCK) {
                                            //if RCK is set, send ACK back
                                            let message_origin: Option<String> =
                                                match message.origin.as_ref() {
                                                    Some(origin) => Some(origin.clone()),
                                                    None => None,
                                                };
                                            //Prepare message
                                            let mut message: OctopipesMessage =
                                                OctopipesMessage::new(
                                                    &client.version,
                                                    &Some(client.id.clone()),
                                                    &message_origin,
                                                    message.ttl,
                                                    OctopipesOptions::ACK,
                                                    0,
                                                    vec![],
                                                );
                                            //Encode message
                                            match serializer::encode_message(&mut message) {
                                                Ok(data_out) => {
                                                    //Write message to CAP
                                                    let _ = pipes::pipe_write(
                                                        &client.tx_pipe.as_ref().unwrap(),
                                                        5000,
                                                        data_out,
                                                    );
                                                }
                                                Err(..) => { /*Ignore error*/ }
                                            }
                                        }
                                        //Call callback
                                        match client.on_received_fn {
                                            Some(on_received) => {
                                                (on_received)(Ok(&message));
                                            }
                                            None => {}
                                        }
                                        drop(message);
                                    }
                                    Err(err) => match client.on_received_fn {
                                        Some(on_received) => {
                                            (on_received)(Err(&err));
                                        }
                                        None => {}
                                    },
                                }
                            }
                            Err(_) => match client.on_received_fn {
                                Some(on_received) => {
                                    (on_received)(Err(&OctopipesError::ReadFailed));
                                }
                                None => {}
                            },
                        }
                        drop(client); //Free lock
                        thread::sleep(std::time::Duration::from_millis(100));
                    }
                    //Exit
                }));
                //Set state to running
                client.state = OctopipesState::Running;
                Ok(())
            }
            OctopipesState::Running => Err(OctopipesError::ThreadAlreadyRunning),
            _ => Err(OctopipesError::NotSubscribed),
        }
    }

    /// ###  loop_stop
    ///
    /// `loop_stop` stops the client loop thread
    /// ```
    pub fn loop_stop(&mut self) -> Result<(), OctopipesError> {
        let mut client = self.this.lock().unwrap();
        match client.state {
            OctopipesState::Running => {
                //Stop thread
                client.state = OctopipesState::Stopped;
                //Take joinable out of Option and then Join thread (NOTE: Using take prevents errors!)
                client.client_loop.take().map(thread::JoinHandle::join);
                Ok(())
            }
            _ => Ok(()),
        }
    }

    //Subscribption functions

    /// ###  subscribe
    ///
    /// `subscribe` subscribe to Octopipes server; the client will subscribe to the groups described in the subscription_list

    pub fn subscribe(
        &mut self,
        subscription_list: &Vec<String>,
    ) -> Result<OctopipesCapError, OctopipesError> {
        //Prepare subscribe message
        let payload: Vec<u8> = cap::encode_subscribption(subscription_list);
        //Send message through the CAP
        match self.send_cap(payload) {
            Err(err) => Err(err),
            Ok(..) => {
                let mut client = self.this.lock().unwrap();
                //Wait for ASSIGNMENT
                match pipes::pipe_read(&client.cap_pipe, 5000) {
                    Err(..) => Err(OctopipesError::ReadFailed),
                    Ok(data_in) => {
                        //Parse message
                        match serializer::decode_message(data_in) {
                            Err(err) => Err(err),
                            Ok(response) => {
                                //Check if message type is ASSIGNMENT
                                match cap::get_cap_message_type(&response.data) {
                                    Ok(message_type) => {
                                        match message_type {
                                            OctopipesCapMessage::Assignment => {
                                                //Ok, is an ASSIGNMENT
                                                //Parse assignment params
                                                match cap::decode_assignment(&response.data) {
                                                    Ok((cap_error, pipe_tx, pipe_rx)) => {
                                                        //Assign params
                                                        if cap_error != OctopipesCapError::NoError {
                                                            return Ok(cap_error);
                                                        }
                                                        client.tx_pipe = pipe_tx;
                                                        client.rx_pipe = pipe_rx;
                                                        Ok(OctopipesCapError::NoError)
                                                    }
                                                    Err(err) => Err(err),
                                                }
                                            }
                                            _ => Err(OctopipesError::BadPacket),
                                        }
                                    }
                                    Err(err) => Err(err),
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// ###  unsubscribe
    ///
    /// `unsubscribe` unsubscribe from Octopipes server; if thread is running it will be stopped

    pub fn unsubscribe(&mut self) -> Result<(), OctopipesError> {
        {
            let client = &mut self.this.lock().unwrap();
            if client.state != OctopipesState::Subscribed && client.state != OctopipesState::Running
            {
                return Err(OctopipesError::NotSubscribed);
            }
        }
        //Prepare message
        let payload: Vec<u8> = cap::encode_unsubscribption();
        match self.send_cap(payload) {
            Err(err) => return Err(err),
            Ok(..) => {}
        }
        //Stop loop
        match self.loop_stop() {
            Ok(..) => {}
            Err(err) => return Err(err),
        }
        //Call on unsubscribed
        {
            let client = &mut self.this.lock().unwrap();
            match client.on_unsubscribed_fn {
                Some(on_unsub) => {
                    (on_unsub)();
                }
                None => {}
            }
            //Set state to UNSUBSCRIBED
            client.state = OctopipesState::Unsubscribed;
        }
        Ok(())
    }

    //Send message functions

    /// ###  send_cap
    ///
    /// `send_cap` sends a message to server through the CAP

    fn send_cap(&self, payload: Vec<u8>) -> Result<(), OctopipesError> {
        let version: OctopipesProtocolVersion;
        let cap_path: String;
        let origin: String;
        {
            let client = &mut self.this.lock().unwrap();
            version = client.version;
            cap_path = client.cap_pipe.clone();
            origin = client.id.clone();
        }
        //Prepare message
        let mut message: OctopipesMessage = OctopipesMessage::new(
            &version,
            &Some(origin),
            &None,
            0,
            OctopipesOptions::empty(),
            0,
            payload,
        );
        //Encode message
        match serializer::encode_message(&mut message) {
            Ok(data_out) => {
                //Write message to cap
                match pipes::pipe_write(&cap_path, 5000, data_out) {
                    Ok(..) => Ok(()),
                    Err(..) => Err(OctopipesError::WriteFailed),
                }
            }
            Err(err) => Err(err),
        }
    }

    /// ###  send
    ///
    /// `send` sends a message to a certain remote

    pub fn send(&self, remote: &String, data: Vec<u8>) -> Result<(), OctopipesError> {
        self.send_ex(remote, data, 0, OctopipesOptions::empty())
    }

    /// ###  send_ex
    ///
    /// `send_ex` sends a message to a certain remote with extended options

    pub fn send_ex(
        &self,
        remote: &String,
        data: Vec<u8>,
        ttl: u8,
        options: OctopipesOptions,
    ) -> Result<(), OctopipesError> {
        let version: OctopipesProtocolVersion;
        let tx_pipe: String;
        let origin: String;
        {
            let client = self.this.lock().unwrap();
            if client.state != OctopipesState::Running && client.state != OctopipesState::Subscribed
            {
                return Err(OctopipesError::NotSubscribed);
            }
            if client.tx_pipe.as_ref().is_none() {
                return Err(OctopipesError::NotSubscribed);
            }
            version = client.version;
            tx_pipe = client.tx_pipe.as_ref().unwrap().clone();
            origin = client.id.clone();
        }
        //Prepare message
        let mut message: OctopipesMessage = OctopipesMessage::new(
            &version,
            &Some(origin),
            &Some(remote.clone()),
            ttl,
            options,
            0,
            data,
        );
        //Encode message
        match serializer::encode_message(&mut message) {
            Ok(data_out) => {
                //Write message to cap
                match pipes::pipe_write(&tx_pipe, 5000, data_out) {
                    Ok(..) => {
                        //If on sent callback is set, call on sent
                        {
                            let client = self.this.lock().unwrap();
                            if client.on_sent_fn.as_ref().is_some() {
                                (client.on_sent_fn.as_ref().unwrap())(&message);
                            }
                        }
                        Ok(())
                    }
                    Err(..) => Err(OctopipesError::WriteFailed),
                }
            }
            Err(err) => Err(err),
        }
    }

    //Callbacks setters

    /// ###  set_on_received_callback
    ///
    /// `set_on_received_callback` sets the function to call on message received
    pub fn set_on_received_callback(
        &mut self,
        callback: fn(Result<&OctopipesMessage, &OctopipesError>),
    ) {
        let mut client = self.this.lock().unwrap();
        client.on_received_fn = Some(callback);
    }

    /// ###  set_on_sent_callbacl
    ///
    /// `set_on_sent_callbacl` sets the function to call when a message is sent
    pub fn set_on_sent_callback(&mut self, callback: fn(&OctopipesMessage)) {
        let mut client = self.this.lock().unwrap();
        client.on_sent_fn = Some(callback);
    }

    /// ###  set_on_subscribed
    ///
    /// `set_on_subscribed` sets the function to call on a successful subscription to the Octopipes Server
    pub fn set_on_subscribed(&mut self, callback: fn()) {
        let mut client = self.this.lock().unwrap();
        client.on_subscribed_fn = Some(callback);
    }

    /// ###  set_on_unsubscribed
    ///
    /// `set_on_unsubscribed` sets the function to call on a successful unsubscription from Octopipes server
    pub fn set_on_unsubscribed(&mut self, callback: fn()) {
        let mut client = self.this.lock().unwrap();
        client.on_unsubscribed_fn = Some(callback);
    }

    //@! Getters
    pub fn get_id(&self) -> String {
        let client = self.this.lock().unwrap();
        client.id.clone()
    }

    pub fn get_cap(&self) -> String {
        let client = self.this.lock().unwrap();
        client.cap_pipe.clone()
    }

    pub fn get_pipe_tx(&self) -> Option<String> {
        let client = self.this.lock().unwrap();
        match client.tx_pipe {
            Some(ref pipe) => Some(pipe.clone()),
            None => None,
        }
    }

    pub fn get_pipe_rx(&self) -> Option<String> {
        let client = self.this.lock().unwrap();
        match client.rx_pipe {
            Some(ref pipe) => Some(pipe.clone()),
            None => None,
        }
    }
}

impl Client {
    /// ### OctopipesClient Constructor
    ///
    /// `new` is constructor for OctopipesClient
    fn new(client_id: String, cap_pipe: String, version: OctopipesProtocolVersion) -> Client {
        Client {
            id: client_id,
            version: version,
            cap_pipe: cap_pipe,
            tx_pipe: None,
            rx_pipe: None,
            state: OctopipesState::Initialized,
            client_loop: None,
            on_received_fn: None,
            on_sent_fn: None,
            on_subscribed_fn: None,
            on_unsubscribed_fn: None,
        }
    }
}

impl Drop for OctopipesClient {
    fn drop(&mut self) {
        //Stop thread
        match self.loop_stop() {
            Ok(_) => drop(self),
            Err(error) => panic!(error), //Don't worry, it won't panic
        }
    }
}
