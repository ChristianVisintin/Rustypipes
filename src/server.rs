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
use super::OctopipesMessage;
use super::OctopipesOptions;
use super::OctopipesProtocolVersion;
use super::OctopipesServer;
use super::OctopipesServerError;
use super::OctopipesServerState;
use super::OctopipesServerWorker;
use super::Subscription;

use super::cap;
use super::pipes;
use super::serializer;

use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

impl OctopipesServer {
    /// ###  new
    ///
    /// `new` instances a new OctopipesServer
    pub fn new(version: OctopipesProtocolVersion, cap_pipe: String, client_folder: String) -> OctopipesServer {
        OctopipesServer {
            version: version,
            state: Arc::new(Mutex::new(OctopipesServerState::Initialized)),
            cap_pipe: cap_pipe,
            client_folder: client_folder,
            cap_receiver: None,
            cap_listener: None,
            workers: Vec::new(),
        }
    }

    //@! CAP

    /// ###  start_cap_listener
    ///
    /// `start_cap_listener` Start CAP listener thread
    pub fn start_cap_listener(&mut self) -> Result<(), OctopipesServerError> {
        //Check if thread is already running
        if self.cap_listener.is_some() {
            return Err(OctopipesServerError::ThreadAlreadyRunning);
        }
        //Create CAP copy
        let cap_pipe: String = self.cap_pipe.clone();
        //Create CAP
        if let Err(_) = pipes::pipe_create(&cap_pipe) {
            return Err(OctopipesServerError::OpenFailed);
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
            let mut terminate_thread: bool = false;
            while !terminate_thread {
                {
                    let current_server_state = server_state_clone.lock().unwrap();
                    //If state is not Runnning, exit
                    match *current_server_state {
                        OctopipesServerState::Block => {
                            //Keep iterating
                            drop(current_server_state); //Allow main thread to change state
                            thread::sleep(Duration::from_millis(100));
                            continue;
                        }
                        OctopipesServerState::Running => {}
                        _ => {
                            terminate_thread = true;
                            continue;
                        }
                    }
                }
                //Listen on CAP
                if let Ok(data_in) = pipes::pipe_read(&cap_pipe, 500) {
                    if data_in.len() == 0 {
                        thread::sleep(Duration::from_millis(100));
                        continue;
                    }
                    //Parse message
                    match serializer::decode_message(data_in) {
                        Err(err) => {
                            if let Err(_) = cap_sender.send(Err(err)) {
                                break; //Terminate thread
                            }
                        }
                        Ok(message) => {
                            //Send CAP message
                            if let Err(_) = cap_sender.send(Ok(message)) {
                                break; //Terminate thread
                            }
                        }
                    }
                    thread::sleep(Duration::from_millis(100));
                }
            }
        }));
        Ok(())
    }

    /// ###  stop_cap_listener
    ///
    /// `stop_cap_listener` stops the server cap listener thread
    pub fn stop_cap_listener(&mut self) -> Result<(), OctopipesServerError> {
        //Set server to running
        let mut server_state = self.state.lock().unwrap();
        match *server_state {
            OctopipesServerState::Running => {
                *server_state = OctopipesServerState::Stopped;
                drop(server_state); //Otherwise other thread will never read the stopped state
                                    //Take joinable out of Option and then Join thread (NOTE: Using take prevents errors!)
                self.cap_listener.take().map(thread::JoinHandle::join);
                //Delete CAP
                let _ = pipes::pipe_delete(&self.cap_pipe);
                Ok(())
            }
            _ => Ok(()),
        }
    }

    //@! Workers

    /// ###  start_worker
    ///
    /// `start_worker` add and starts a new worker for the Octopipes Server. The server must be in Running state
    pub fn start_worker(
        &mut self,
        client: String,
        subscriptions: Vec<String>,
        cli_tx_pipe: String,
        cli_rx_pipe: String,
    ) -> Result<(), OctopipesServerError> {
        //State must be already started
        {
            let server_state = self.state.lock().unwrap();
            if *server_state != OctopipesServerState::Running {
                return Err(OctopipesServerError::Uninitialized);
            }
        }
        //Check if a worker with that name already exists
        if self.worker_exists(&client) {
            return Err(OctopipesServerError::WorkerExists);
        }
        //Instance new worker
        match OctopipesServerWorker::new(client, subscriptions, cli_tx_pipe, cli_rx_pipe) {
            Ok(new_worker) => {
                //Push new worker
                self.workers.push(new_worker);
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    /// ###  stop_worker
    ///
    /// `stop_worker` stops a running worker for the Octopipes Server. The server must be in Running state
    pub fn stop_worker(&mut self, client: String) -> Result<(), OctopipesServerError> {
        //Look for worker in workers
        let mut item: usize = 0;
        for worker in self.workers.iter_mut() {
            if worker.client_id == client {
                let result = worker.stop_worker();
                drop(worker);
                //Remove worker from workers
                self.workers.remove(item);
                return result;
            }
            item += 1;
        }
        Err(OctopipesServerError::WorkerNotFound)
    }

    /// ### dispatch_message
    ///
    /// `dispatch_message` Dispatch a message to subscribed nodes. Returns error with the error type and the client id of the worker associated which returned an error
    pub fn dispatch_message(
        &self,
        message: &OctopipesMessage,
    ) -> Result<(), (Option<String>, OctopipesServerError)> {
        //Found worker where to dispatch the message
        if message.remote.is_none() {
            return Err((None, OctopipesServerError::NoRecipient));
        }
        let recipient: String = message.remote.as_ref().unwrap().clone();
        let workers_associated: Vec<&OctopipesServerWorker> = self.match_subscription(&recipient);
        //For each associated worker, send the message out
        for worker in workers_associated {
            if let Err(error) = worker.send(message) {
                return Err((Some(worker.client_id.clone()), error));
            }
        }
        Ok(())
    }
    //@! Management
    //TODO: manage_subscription / manage_unsubscription

    //@! Privates

    /// ### match_subscription
    ///
    /// `match_subscription` Returns the Workers associated to a certain subscription
    fn match_subscription(&self, subscription: &String) -> Vec<&OctopipesServerWorker> {
        let mut subject_workers: Vec<&OctopipesServerWorker> = Vec::new();
        for worker in &self.workers {
            if worker.is_subscribed(subscription) {
                subject_workers.push(worker);
            }
        }
        subject_workers
    }

    /// ###  worker_exists
    ///
    /// `worker_exists` Checks whether a Worker with that name already exists
    fn worker_exists(&self, worker_name: &String) -> bool {
        for worker in &self.workers {
            if worker.client_id == *worker_name {
                return true;
            }
        }
        false
    }
}

impl Drop for OctopipesServer {
    fn drop(&mut self) {
        //Stop workers
        for worker in self.workers.iter_mut() {
            let _ = worker.stop_worker();
        }
        //Stop thread
        let _ = self.stop_cap_listener();
        drop(self);
    }
}

impl OctopipesServerWorker {
    /// ###  new
    ///
    /// `new` instances a new OctopipesServerWorker
    fn new(
        client_id: String,
        subscriptions: Vec<String>,
        cli_pipe_tx: String,
        cli_pipe_rx: String,
    ) -> Result<OctopipesServerWorker, OctopipesServerError> {
        //Prepare subscriptions
        let subscriptions_obj = Subscription::new(subscriptions);
        //Create pipes
        if let Err(..) = pipes::pipe_delete(&cli_pipe_rx) {
            return Err(OctopipesServerError::OpenFailed);
        }
        if let Err(..) = pipes::pipe_delete(&cli_pipe_tx) {
            return Err(OctopipesServerError::OpenFailed);
        }
        //Prepare thread stuff
        let pipe_read: String = cli_pipe_tx.clone();
        let pipe_write: String = cli_pipe_rx.clone();
        let worker_active: Arc<Mutex<bool>> = Arc::new(Mutex::new(true)); //True
        let thread_active: Arc<Mutex<bool>> = Arc::clone(&worker_active); //Clone active for thread
                                                                          //Create channel
        let (worker_sender, worker_receiver) = mpsc::channel();
        //Start thread
        let join_handle = thread::spawn(move || {
            //TODO: implement thread
            let mut terminate_thread: bool = false;
            while !terminate_thread {
                {
                    //Check if thread has to be stopped
                    let active = thread_active.lock().unwrap();
                    if *active == false {
                        terminate_thread = true;
                    }
                }
            }
            //NOTE: Move sender here
        });
        //Instance and return a new OctopipesServerWorker
        Ok(OctopipesServerWorker {
            client_id: client_id,
            subscription: subscriptions_obj,
            pipe_read: cli_pipe_tx,  //Invert pipes for Server
            pipe_write: cli_pipe_rx, //Invert pipes for Server
            worker_loop: Some(join_handle),
            worker_active: worker_active,
            receiver: worker_receiver,
        })
    }

    /// ###  stop_worker
    ///
    /// `stop_worker` stops worker
    fn stop_worker(&mut self) -> Result<(), OctopipesServerError> {
        {
            let mut active = self.worker_active.lock().unwrap();
            *active = false;
        }
        //Stop thread
        if self.worker_loop.is_some() {
            self.worker_loop.take().map(thread::JoinHandle::join);
            //Delete pipes
            let _ = pipes::pipe_delete(&self.pipe_read);
            let _ = pipes::pipe_delete(&self.pipe_write);
            Ok(())
        } else {
            Err(OctopipesServerError::WorkerNotRunning)
        }
    }

    /// ### send
    ///
    /// `send` sends the provided message to the client
    pub fn send(&self, message: &OctopipesMessage) -> Result<(), OctopipesServerError> {
        //Encode message
        match serializer::encode_message(message) {
            Err(..) => Err(OctopipesServerError::BadPacket),
            Ok(data_out) => {
                //Write data
                let timeout: u128 = message.ttl as u128 * 1000;
                match pipes::pipe_write(&self.pipe_write, timeout, data_out) {
                    Ok(..) => Ok(()),
                    Err(..) => Err(OctopipesServerError::WriteFailed),
                }
            }
        }
    }

    /// ### is_subscribed
    ///
    /// `is_subscribed` returns wether groups contains the provided string
    fn is_subscribed(&self, subscription: &String) -> bool {
        self.subscription.is_subscribed(subscription)
    }
}

impl Drop for OctopipesServerWorker {
    fn drop(&mut self) {
        //Stop thread
        let _ = self.stop_worker();
        drop(self);
    }
}

impl Subscription {
    /// ###  new
    ///
    /// `new` instances a new Subscription
    fn new(subscriptions: Vec<String>) -> Subscription {
        Subscription {
            groups: subscriptions,
            subscription_time: std::time::Instant::now(),
        }
    }

    /// ###  is_subscribed
    ///
    /// `is_subscribed` returns wether groups contains the provided string
    fn is_subscribed(&self, to_find: &String) -> bool {
        self.groups.contains(to_find)
    }
}
