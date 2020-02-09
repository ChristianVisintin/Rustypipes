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
                                                        match message.get_origin() {
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
                                        match cap::get_cap_message_type(response.get_data()) {
                                            Ok(message_type) => {
                                                match message_type {
                                                    OctopipesCapMessage::Assignment => {
                                                        //Ok, is an ASSIGNMENT
                                                        //Parse assignment params
                                                        match cap::decode_assignment(response.get_data()) {
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

#[cfg(test)]
mod tests {
    use super::*;

    //Refer to production test to have a working environment
    #[test]
    fn test_client_not_working() {
        let cap_pipe: String = String::from("/tmp/test-client-cap.pipe");
        //Create cap pipe
        pipes::pipe_create(&cap_pipe).expect("Could not create CAP pipe");
        let mut client: OctopipesClient = OctopipesClient::new(String::from("myclient123"), cap_pipe.clone(), OctopipesProtocolVersion::Version1);
        //Set callbacks
        client.set_on_received_callback(on_received);
        client.set_on_sent_callback(on_sent);
        client.set_on_subscribed(on_subscribed);
        client.set_on_unsubscribed(on_unsubscribed);
        //Try to start loop before subscribing
        if let Ok(_) = client.loop_start() {
            panic!("Client started loop before subscribing");
        }
        //Try to subscribe
        if let Ok(_) = client.subscribe(&vec![String::from("BROADCAST")]) {
            panic!("Subscribe returned OK, but there's no server serving");
        }
        //Try to send message
        if let Ok(_) = client.send(&String::from("foobar"), vec![0x01, 0x02, 0x03]) {
            panic!("Could not send message");
        }
        //Try to process message
        if let Ok(_) = client.get_next_message() {
            panic!("Get next message should have returned error");
        }
        if let Err(err) = client.loop_stop() {
            panic!("loop stop returned error: {}", err);
        }
        //Delete pipe
        pipes::pipe_delete(&cap_pipe).expect("Could not delete CAP pipe");
    }

    use std::sync::{Arc, Mutex};
    use std::thread::{sleep, spawn, JoinHandle};
    use std::time::{Duration, Instant};
    #[test]
    fn client_with_server() {
        //Simulates an entire server with a client
        let cap_pipe: String = String::from("/tmp/test-client-cap.fifo");
        let cap_pipe_r: String = cap_pipe.clone();
        let client_folder: String = String::from("/tmp/test-clients/");
        //Instance Server
        let mut server: crate::OctopipesServer = crate::OctopipesServer::new(
            crate::OctopipesProtocolVersion::Version1,
            cap_pipe,
            client_folder,
        );
        //Set server callbacks
        server.set_on_subscription(on_subscribption);
        server.set_on_unsubscription(on_unsubscription);
        //@! Start server
        if let Err(error) = server.start_cap_listener() {
            panic!("Could not start CAP listener: {}", error);
        }
        println!("CAP Listener started...");
        let client_terminate: bool = false;
        let client_terminate_arc: Arc<Mutex<bool>> = Arc::new(Mutex::new(client_terminate));
        let client_terminate_arc2: Arc<Mutex<bool>> = Arc::clone(&client_terminate_arc);
        //Start client
        let client_join_hnd: JoinHandle<()> = spawn(move || {
            println!("Client_r thread started");
            //This client subscribes to BROADCAST and then wait for an incoming message from client_w
            let t_start: Instant = Instant::now();
            let cap_pipe_w: String = cap_pipe_r.clone();
            let mut client_r: crate::OctopipesClient = crate::OctopipesClient::new(
                String::from("test_client_r"),
                cap_pipe_r,
                crate::OctopipesProtocolVersion::Version1,
            );
            let groups: Vec<String> = vec![String::from("TestClient"), String::from("BROADCAST")];
            match client_r.subscribe(&groups) {
                Ok(cap_error) => {
                    match cap_error {
                        crate::OctopipesCapError::NoError => {
                            println!("Client_r subscribed (no CAP error)");
                        },
                        _ => panic!("Client_r couldn't subscribe, CAP error: {}\n", cap_error)
                    }
                },
                Err(error) => panic!("Error while client_r was trying to subscribe to server: {}\n", error)
            }
            println!(
                "It took {}ms for client_r to subscribe",
                t_start.elapsed().as_millis()
            );
            //Start client loop
            if let Err(error) = client_r.loop_start() {
                panic!("Couldn't start client_r loop: {}\n", error);
            }
            //@! Start WRITE client
            let client_w_join_hnd: JoinHandle<()> = spawn(move || {
                println!("Client_w thread started");
                //This client subscribes and then sends a message to test_client_r
                let t_start: Instant = Instant::now();
                let mut client_w: crate::OctopipesClient = crate::OctopipesClient::new(
                    String::from("test_client_w"),
                    cap_pipe_w,
                    crate::OctopipesProtocolVersion::Version1,
                );
                match client_w.subscribe(&vec![]) {
                    Ok(cap_error) => {
                        match cap_error {
                            crate::OctopipesCapError::NoError => {
                                println!("Client_w subscribed (no CAP error)");
                            },
                            _ => panic!("Client_w couldn't subscribe, CAP error: {}\n", cap_error)
                        }
                    },
                    Err(error) => panic!("Error while client_w was trying to subscribe to server: {}\n", error)
                }
                println!(
                    "It took {}ms for client_w to subscribe",
                    t_start.elapsed().as_millis()
                );
                //Send a message
                let t_subscribed: Instant = Instant::now();
                if let Err(error) = client_w.send(
                    &String::from("TestClient"),
                    vec!['H' as u8, 'E' as u8, 'L' as u8, 'L' as u8, 'O' as u8],
                ) {
                    panic!(
                        "Error while trying to send 'HELLO' to test_client_r: {}\n",
                        error
                    );
                }
                println!(
                    "It took {}ms for client_w to send a message",
                    t_subscribed.elapsed().as_millis()
                );
                //Unsubscribe client_w
                let t_sent: Instant = Instant::now();
                if let Err(error) = client_w.unsubscribe() {
                    panic!("Error while client_w was trying to unsubscribe: {}", error);
                }
                println!(
                    "It took {}ms for client_w to unsubscribe",
                    t_sent.elapsed().as_millis()
                );
                println!(
                    "Client_w terminated after {}ms",
                    t_start.elapsed().as_millis()
                );
                //@! End of test_client_w thread
            });
            //Wait for client message
            let mut message_received: bool = false;
            println!("Client_r is now waiting for incoming messages...");
            while !message_received {
                match client_r.get_all_message() {
                    Err(error) => {
                        panic!("Error while trying to get messages on client_r: {}\n", error)
                    }
                    Ok(messages) => {
                        for message in messages.iter() {
                            println!(
                                "Received message from {}: {:?}",
                                message.get_origin().unwrap(),
                                message.get_data()
                            );
                            assert_eq!(
                                *message.get_origin().unwrap(),
                                String::from("test_client_w"),
                                "Received message origin should be 'test_client_w', but is {}",
                                message.get_origin().unwrap()
                            );
                        }
                        if messages.len() > 0 {
                            message_received = true;
                        }
                    }
                }
            }
            //Unsubscribe
            let t_recv: Instant = Instant::now();
            if let Err(error) = client_r.unsubscribe() {
                panic!("Error while client_r was trying to unsubscribe: {}\n", error);
            }
            println!(
                "It took {}ms for client_w to unsubscribe",
                t_recv.elapsed().as_millis()
            );
            //Join client w before exiting
            if let Err(err) = client_w_join_hnd.join() {
                panic!("Client W thread panic: {:?}", err);
            }
            println!(
                "Client_r terminated after {}ms",
                t_start.elapsed().as_millis()
            );
            let mut terminated = client_terminate_arc2.lock().unwrap();
            *terminated = true;
            //@! End of test_client_r thread
        });
        sleep(Duration::from_millis(100));
        //Set a timer (10s)
        let t_start_loop: Instant = Instant::now();
        //Wait for both clients subscription
        while (server.is_subscribed(String::from("test_client_r")).is_none() || server.is_subscribed(String::from("test_client_w")).is_none()) && t_start_loop.elapsed().as_millis() < 10000 {
            //Process cap
            if let Err(error) = server.process_cap_all() {
                panic!("Error while processing CAP: {}\n", error);
            }
            sleep(Duration::from_millis(100)); //Sleep for 100 ms
        }
        if t_start_loop.elapsed().as_millis() >= 10000 {
            panic!("Client couldn't subscribe (TIMEOUT)\n");
        }
        println!("OK, 'test_client_r and test_client_w' is subscribed");
        //Verify if client is subscribed
        let clients: Vec<String> = server.get_clients();
        assert_eq!(clients.len(), 2, "Clients are not subscribed (clients subscribed: {})", clients.len());
        //Verify test_client_r subscriptions
        match server.get_subscriptions(String::from("test_client_r")) {
            Some(groups) => {
                //Subscriptions should be 'TestClient' 'BROADCAST' and 'test_client_r' (cause implicit)
                assert_eq!(
                    groups[0],
                    String::from("TestClient"),
                    "Groups[0] should be 'TestClient' but is {}",
                    groups[0]
                );
                assert_eq!(
                    groups[1],
                    String::from("BROADCAST"),
                    "Groups[1] should be 'BROADCAST' but is {}",
                    groups[1]
                );
                assert_eq!(
                    groups[2],
                    String::from("test_client_r"),
                    "Groups[1] should be 'test_client_r' but is {}",
                    groups[2]
                );
            }
            None => panic!("'test_client_r' is subscribed to nothing\n"),
        }
        //Verify test_client_w subscription
        match server.get_subscriptions(String::from("test_client_w")) {
            Some(groups) => {
                //Subscriptions should be 'test_client_w' (cause implicit)
                assert_eq!(
                    groups[0],
                    String::from("test_client_w"),
                    "Groups[0] should be 'test_client_w' but is {}",
                    groups[0]
                );
            }
            None => panic!("'test_client_w' is subscribed to nothing\n"),
        }
        //@! Listen for client
        let t_start_loop = Instant::now();
        //Timeout 10 seconds
        while t_start_loop.elapsed().as_millis() < 10000 {
            //Listen for incoming CAP messages
            if let Err(error) = server.process_cap_all() {
                println!("Error while processing CAP: {}", error);
            }
            //Process workers
            if let Err((worker, error)) = server.process_all() {
                println!("Error while trying to process client {}: {}", worker, error);
            }
            let terminated = client_terminate_arc.lock().unwrap();
            if *terminated {
                break; //Break if client terminated
            }
            sleep(Duration::from_millis(100)); //Sleep for 100ms
        }
        //Force client interruption
        let mut terminated = client_terminate_arc.lock().unwrap();
        *terminated = true;
        println!(
            "It took {}ms to process all the two clients!",
            t_start_loop.elapsed().as_millis()
        );
        //Terminate client
        if let Err(err) = client_join_hnd.join() {
            panic!("Client R thread panic: {:?}\n", err);
        }
        //Stop server
        if let Err(error) = server.stop_server() {
            panic!("Could not stop Server: {}\n", error);
        }
    }

    //Callbacks
    fn on_subscribption(client_id: String) {
        println!("ON_SUBSCRIBPTION_CALLBACK - Client {} subscribed!", client_id);
    }

    fn on_unsubscription(client_id: String) {
        println!("ON_UNSUBSCRIBPTION_CALLBACK - Client {} unsubscribed!", client_id);
    }

    //Callbacks
    fn on_received(recv: Result<&OctopipesMessage, &OctopipesError>) {
        match recv {
            Ok(_) => {
                println!("Received message!");
            },
            Err(error) => {
                println!("Received error: {}", error);
            }
        }
    }

    fn on_sent(_message: &OctopipesMessage) {
        println!("Message sent");
    }

    fn on_subscribed() {
        println!("Client subscribed");
    }

    fn on_unsubscribed() {
        println!("Client unsubscribed");
    }
}
