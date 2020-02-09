//! ## Client
//!
//! `client` is the module which takes care of managing the OctopipesClient struct and
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

use crate::OctopipesCapError;
use crate::OctopipesCapMessage;
use crate::OctopipesClient;
use crate::OctopipesError;
use crate::OctopipesMessage;
use crate::OctopipesOptions;
use crate::OctopipesProtocolVersion;
use crate::OctopipesState;

use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use crate::cap;
use crate::pipes;
use crate::serializer;

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
            id: client_id,
            version: version,
            cap_pipe: cap_pipe,
            tx_pipe: None,
            rx_pipe: None,
            state: Arc::new(Mutex::new(OctopipesState::Initialized)),
            client_loop: None,
            client_receiver: None,
            on_received_fn: None,
            on_sent_fn: None,
            on_subscribed_fn: None,
            on_unsubscribed_fn: None,
        }
    }

    //Thread operations

    /// ###  loop_start
    ///
    /// `loop_start` starts the client loop thread which checks if new messages are available
    pub fn loop_start(&mut self) -> Result<(), OctopipesError> {
        let mut client_state = self.state.lock().unwrap();
        match *client_state {
            OctopipesState::Subscribed => {
                //Create threaded client
                if self.rx_pipe.is_none() || self.tx_pipe.is_none() {
                    return Err(OctopipesError::Uninitialized);
                }
                //Set state to running
                *client_state = OctopipesState::Running;
                let this_state_rc = Arc::clone(&self.state);
                let rx_pipe: String = self.rx_pipe.as_ref().unwrap().clone();
                let tx_pipe: String = self.tx_pipe.as_ref().unwrap().clone();
                let version: OctopipesProtocolVersion = self.version;
                let client_id: String = self.id.clone();
                let (client_sender, client_receiver) = mpsc::channel();
                self.client_receiver = Some(client_receiver);
                self.client_loop = Some(thread::spawn(move || {
                    let mut terminate_thread: bool = false;
                    while !terminate_thread {
                        {
                            let current_state = this_state_rc.lock().unwrap();
                            if *current_state != OctopipesState::Running {
                                terminate_thread = true;
                            }
                        }
                        //Try to read (Read for 500 ms and sleep for 100ms)
                        match pipes::pipe_read(&rx_pipe, 500) {
                            Ok(data) => {
                                match data {
                                    None => {
                                        thread::sleep(std::time::Duration::from_millis(100));
                                        continue; //Just go on
                                    },
                                    Some(data) => {
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
                                                            &version,
                                                            &Some(client_id.clone()),
                                                            &message_origin,
                                                            message.ttl,
                                                            OctopipesOptions::ACK,
                                                            vec![],
                                                        );
                                                    //Encode message
                                                    match serializer::encode_message(&mut message) {
                                                        Ok(data_out) => {
                                                            //Write message to CAP
                                                            let _ =
                                                                pipes::pipe_write(&tx_pipe, 5000, data_out);
                                                        }
                                                        Err(..) => { /*Ignore error*/ }
                                                    }
                                                }
                                                //Send message
                                                if let Err(_) = client_sender.send(Ok(message)) {
                                                    break; //Terminate thread
                                                }
                                            }
                                            Err(err) => {
                                                if let Err(_) = client_sender.send(Err(err)) {
                                                    break; //Terminate thread
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Err(_) => {
                                if let Err(_) = client_sender.send(Err(OctopipesError::ReadFailed))
                                {
                                    break; //Terminate thread
                                }
                            }
                        }
                    }
                    thread::sleep(std::time::Duration::from_millis(100));
                    //Exit
                }));
                Ok(())
            }
            OctopipesState::Running => Err(OctopipesError::ThreadAlreadyRunning),
            _ => Err(OctopipesError::NotSubscribed),
        }
    }

    /// ###  loop_stop
    ///
    /// `loop_stop` stops the client loop thread
    pub fn loop_stop(&mut self) -> Result<(), OctopipesError> {
        let mut client_state = self.state.lock().unwrap();
        match *client_state {
            OctopipesState::Running => {
                //Stop thread
                *client_state = OctopipesState::Stopped;
                drop(client_state); //Otherwise the other thread will never read the state
                
                //Take joinable out of Option and then Join thread (NOTE: Using take prevents errors!)
                self.client_loop.take().map(thread::JoinHandle::join);
                Ok(())
            }
            _ => Ok(()),
        }
    }

    //subscription functions

    /// ###  subscribe
    ///
    /// `subscribe` subscribe to Octopipes server; the client will subscribe to the groups described in the subscription_list

    pub fn subscribe(
        &mut self,
        subscription_list: &Vec<String>,
    ) -> Result<OctopipesCapError, OctopipesError> {
        //Prepare subscribe message
        let payload: Vec<u8> = cap::encode_subscription(subscription_list);
        //Send message through the CAP
        match self.send_cap(payload) {
            Err(err) => Err(err),
            Ok(..) => {
                //Wait for ASSIGNMENT
                match pipes::pipe_read(&self.cap_pipe, 5000) {
                    Err(..) => Err(OctopipesError::ReadFailed),
                    Ok(data_in) => {
                        match data_in {
                            None => return Err(OctopipesError::NoDataAvailable),
                            Some(data_in) => {
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
                                                                self.tx_pipe = pipe_tx;
                                                                self.rx_pipe = pipe_rx;
                                                                let mut client_state =
                                                                    self.state.lock().unwrap();
                                                                *client_state = OctopipesState::Subscribed;
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
        }
    }

    /// ###  unsubscribe
    ///
    /// `unsubscribe` unsubscribe from Octopipes server; if thread is running it will be stopped

    pub fn unsubscribe(&mut self) -> Result<(), OctopipesError> {
        {
            let client_state = self.state.lock().unwrap();
            if *client_state != OctopipesState::Subscribed
                && *client_state != OctopipesState::Running
            {
                return Err(OctopipesError::NotSubscribed);
            }
        }
        //Prepare message
        let payload: Vec<u8> = cap::encode_unsubscription();
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
        match self.on_unsubscribed_fn {
            Some(on_unsub) => {
                (on_unsub)();
            }
            None => {}
        }
        //Set state to UNSUBSCRIBED
        let mut client_state = self.state.lock().unwrap();
        *client_state = OctopipesState::Unsubscribed;
        Ok(())
    }

    //Send message functions

    /// ###  send_cap
    ///
    /// `send_cap` sends a message to server through the CAP

    fn send_cap(&self, payload: Vec<u8>) -> Result<(), OctopipesError> {
        //Prepare message
        let mut message: OctopipesMessage = OctopipesMessage::new(
            &self.version,
            &Some(self.id.clone()),
            &None,
            60,
            OctopipesOptions::empty(),
            payload,
        );
        //Encode message
        match serializer::encode_message(&mut message) {
            Ok(data_out) => {
                //Write message to cap
                match pipes::pipe_write(&self.cap_pipe, 5000, data_out) {
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
        {
            let client_state = self.state.lock().unwrap();
            if *client_state != OctopipesState::Running
                && *client_state != OctopipesState::Subscribed
            {
                return Err(OctopipesError::NotSubscribed);
            }
            if self.tx_pipe.is_none() {
                return Err(OctopipesError::NotSubscribed);
            }
        }
        //Prepare message
        let mut message: OctopipesMessage = OctopipesMessage::new(
            &self.version,
            &Some(self.id.clone()),
            &Some(remote.clone()),
            ttl,
            options,
            data,
        );
        //Encode message
        match serializer::encode_message(&mut message) {
            Ok(data_out) => {
                //Write message to cap
                match pipes::pipe_write(&self.tx_pipe.as_ref().unwrap(), 5000, data_out) {
                    Ok(..) => {
                        //If on sent callback is set, call on sent
                        {
                            if self.on_sent_fn.as_ref().is_some() {
                                (self.on_sent_fn.as_ref().unwrap())(&message);
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

    //@! Message readers
    /// ###  get_next_message
    ///
    /// `get_next_message` Gets the next available message on the receiver
    /// If a message is available Ok(message) is returned
    /// If no message is available Ok(None) is returned
    /// If there was an error while reading inbox, Err(OctopipesError) is returned
    pub fn get_next_message(&self) -> Result<Option<OctopipesMessage>, OctopipesError> {
        {
            //Check if thread is running
            let current_state = self.state.lock().unwrap();
            if *current_state != OctopipesState::Running {
                return Err(OctopipesError::Uninitialized);
            }
        }
        //Try receive
        match self.client_receiver.as_ref() {
            None => Err(OctopipesError::Uninitialized),
            Some(receiver) => match receiver.try_recv() {
                Ok(payload) => match payload {
                    Ok(message) => Ok(Some(message)),
                    Err(error) => Err(error),
                },
                Err(error) => match error {
                    mpsc::TryRecvError::Empty => Ok(None),
                    _ => Err(OctopipesError::ThreadError),
                },
            },
        }
    }

    /// ###  get_all_message
    ///
    /// `get_all_message` Gets all the available messages on the receiver
    /// If there was no error while reading the inbox a vector with all the messages is returned (could have length 0)
    /// If there was an error while reading inbox, Err(OctopipesError) is returned
    pub fn get_all_message(&self) -> Result<Vec<OctopipesMessage>, OctopipesError> {
        let mut inbox: Vec<OctopipesMessage> = Vec::new();
        loop {
            match self.get_next_message() {
                Err(error) => return Err(error),
                Ok(ret) => match ret {
                    Some(message) => inbox.push(message),
                    None => break,
                },
            }
        }
        Ok(inbox)
    }

    //Callbacks setters

    /// ###  set_on_received_callback
    ///
    /// `set_on_received_callback` sets the function to call on message received
    pub fn set_on_received_callback(
        &mut self,
        callback: fn(Result<&OctopipesMessage, &OctopipesError>),
    ) {
        self.on_received_fn = Some(callback);
    }

    /// ###  set_on_sent_callbacl
    ///
    /// `set_on_sent_callbacl` sets the function to call when a message is sent
    pub fn set_on_sent_callback(&mut self, callback: fn(&OctopipesMessage)) {
        self.on_sent_fn = Some(callback);
    }

    /// ###  set_on_subscribed
    ///
    /// `set_on_subscribed` sets the function to call on a successful subscription to the Octopipes Server
    pub fn set_on_subscribed(&mut self, callback: fn()) {
        self.on_subscribed_fn = Some(callback);
    }

    /// ###  set_on_unsubscribed
    ///
    /// `set_on_unsubscribed` sets the function to call on a successful unsubscription from Octopipes server
    pub fn set_on_unsubscribed(&mut self, callback: fn()) {
        self.on_unsubscribed_fn = Some(callback);
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
