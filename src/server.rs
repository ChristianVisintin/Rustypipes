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
    pub fn new(
        version: OctopipesProtocolVersion,
        cap_pipe: String,
        client_folder: String,
    ) -> OctopipesServer {
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

    /// ###  stop_server
    ///
    /// `stop_server` stops the octopipes server (workers and cap listener)
    pub fn stop_server(&mut self) -> Result<(), OctopipesServerError> {
        {
            let current_state = self.state.lock().unwrap();
            if *current_state != OctopipesServerState::Running {
                return Ok(())
            }
        }
        //Stop workers
        for worker in self.workers.iter_mut() {
            if let Err(error) = worker.stop_worker() {
                return Err(error)
            }
        }
        //Stop thread
        if let Err(error) = self.stop_cap_listener() {
            return Err(error)
        }
        Ok(())
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
                if let Ok(data_in) = pipes::pipe_read(&cap_pipe, 100) {
                    if data_in.len() == 0 {
                        thread::sleep(Duration::from_millis(100));
                        continue;
                    }
                    //Parse message
                    match serializer::decode_message(data_in) {
                        Err(err) => {
                            if let Err(_) = cap_sender.send(Err(err.to_server_error())) {
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

    /// ###  lock_cap
    ///
    /// `lock_cap` Set Server state to BLOCK in order to stop reading from pipe
    fn lock_cap(&mut self) {
        let mut server_state = self.state.lock().unwrap();
        *server_state = OctopipesServerState::Block;
        thread::sleep(Duration::from_millis(100)); //Give main thread the time to block
    }

    /// ###  unlock_cap
    ///
    /// `unlock_cap` Set server state back to RUNNING in order to allow read from pipe
    fn unlock_cap(&mut self) {
        let mut server_state = self.state.lock().unwrap();
        *server_state = OctopipesServerState::Running;
    }

    /// ###  write_cap
    ///
    /// `write_cap` write a message to the CAP
    fn write_cap(
        &mut self,
        client: &String,
        data_out: Vec<u8>,
    ) -> Result<(), OctopipesServerError> {
        //Block CAP
        self.lock_cap();
        //Prepare message
        let message: OctopipesMessage = OctopipesMessage::new(
            &self.version,
            &None,
            &Some(client.clone()),
            60,
            OctopipesOptions::empty(),
            data_out,
        );
        //Encode message
        match serializer::encode_message(&message) {
            Err(err) => {
                self.unlock_cap();
                Err(err.to_server_error())
            }
            Ok(data) => {
                //Write data out
                match pipes::pipe_write(&self.cap_pipe, 60000, data) {
                    Ok(..) => {
                        //Unlock CAP
                        self.unlock_cap();
                        Ok(())
                    }
                    Err(..) => {
                        //Unlock CAP
                        self.unlock_cap();
                        Err(OctopipesServerError::WriteFailed)
                    }
                }
            }
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
            //Refuse subscription from an already subscribed client
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
    pub fn stop_worker(&mut self, client: &String) -> Result<(), OctopipesServerError> {
        //Look for worker in workers
        let mut item: usize = 0;
        for worker in self.workers.iter_mut() {
            if worker.client_id == *client {
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

    /// ### process_cap_once
    ///
    /// `process_cap_once` Reads up to one message from the CAP receiver and process it.
    /// When Ok, returns the amount of messages processed (0/1), otherwise an Error
    pub fn process_cap_once(&mut self) -> Result<usize, OctopipesServerError> {
        {
            //Check if server is intiialized
            let current_server_state = self.state.lock().unwrap();
            if *current_server_state != OctopipesServerState::Running {
                return Err(OctopipesServerError::Uninitialized);
            }
        }
        if self.cap_receiver.is_none() {
            return Err(OctopipesServerError::Uninitialized);
        }
        let receiver: &mpsc::Receiver<Result<OctopipesMessage, OctopipesServerError>> =
            self.cap_receiver.as_ref().unwrap();
        //Call try recv
        match receiver.try_recv() {
            Ok(received) => {
                match received {
                    Ok(message) => {
                        //Process message
                        match self.manage_cap_message(&message) {
                            Ok(..) => Ok(1),
                            Err(err) => Err(err),
                        }
                    }
                    Err(error) => Err(error), //Otherwise return error
                }
            }
            Err(recv_error) => {
                match recv_error {
                    mpsc::TryRecvError::Empty => Ok(0), //If recv queue is empty return none
                    _ => Err(OctopipesServerError::WorkerNotRunning), //Otherwise return Worker not running
                }
            }
        }
    }

    /// ### process_cap_all
    ///
    /// `process_cap_all` Reads all the available messages on the CAP until no one is available.
    /// When Ok, returns the amount of messages processed, otherwise Error
    pub fn process_cap_all(&mut self) -> Result<usize, OctopipesServerError> {
        let mut amount_of_process: usize = 0;
        loop {
            match self.process_cap_once() {
                Ok(processed_messages) => {
                    if processed_messages > 0 {
                        amount_of_process += processed_messages;
                        continue;
                    } else {
                        break; //No more messages
                    }
                }
                Err(err) => return Err(err),
            }
        }
        Ok(amount_of_process)
    }

    /// ### manage_cap_message
    ///
    /// `manage_cap_message` Takes a Message from CAP and based on its type perform an action to the server.
    /// If necessary it also responds to the client through the CAP.
    /// (e.g. in case of subscription it will send an assignment back)
    pub fn manage_cap_message(
        &mut self,
        message: &OctopipesMessage,
    ) -> Result<OctopipesCapMessage, OctopipesServerError> {
        //Get message origin, if None, return error
        let origin: String;
        match &message.origin {
            None => return Err(OctopipesServerError::NoRecipient),
            Some(client) => origin = client.clone(),
        }
        //Get message type
        match cap::get_cap_message_type(&message.data) {
            Err(err) => Err(err.to_server_error()),
            Ok(cap_message) => {
                match cap_message {
                    OctopipesCapMessage::Subscription => {
                        //Parse subscription message
                        match cap::decode_subscription(&message.data) {
                            Err(err) => Err(err.to_server_error()),
                            Ok(mut groups) => {
                                //@! Very important, add client id to groups
                                groups.push(origin.clone());
                                self.manage_subscription(&origin, &groups)
                            }
                        }
                    }
                    OctopipesCapMessage::Unsubscription => {
                        //Parse unsubscription
                        match cap::decode_unsubscription(&message.data) {
                            Err(err) => Err(err.to_server_error()),
                            Ok(..) => self.manage_unsubscription(&origin),
                        }
                    }
                    _ => Err(OctopipesServerError::BadPacket),
                }
            }
        }
    }

    /// ### manage_subscription
    ///
    /// `manage_subscription` Handle a subscription request. If a worker with this ID is available start a new one and send the assignment back to the client
    fn manage_subscription(
        &mut self,
        client_id: &String,
        groups: &Vec<String>,
    ) -> Result<OctopipesCapMessage, OctopipesServerError> {
        //Check if client is already subsribed
        for worker in &self.workers {
            if *client_id == worker.client_id {
                //Encode assignment with cap error
                let data_out: Vec<u8> =
                    cap::encode_assignment(OctopipesCapError::NameAlreadyTaken, None, None);
                let _ = self.write_cap(client_id, data_out);
                return Err(OctopipesServerError::WorkerExists);
            }
        }
        //Okay, client doesn't exist, assign Pipes
        let tx_pipe: String = self.client_folder.clone() + "/" + client_id + "_tx.fifo";
        let rx_pipe: String = self.client_folder.clone() + "/" + client_id + "_rx.fifo";
        //Start worker
        match self.start_worker(
            client_id.clone(),
            groups.clone(),
            tx_pipe.clone(),
            rx_pipe.clone(),
        ) {
            Err(error) => {
                let data_out: Vec<u8> =
                    cap::encode_assignment(OctopipesCapError::FileSystemError, None, None);
                let _ = self.write_cap(client_id, data_out);
                Err(error)
            }
            Ok(..) => {
                //Encode assignment
                let data_out: Vec<u8> = cap::encode_assignment(
                    OctopipesCapError::NoError,
                    Some(&tx_pipe),
                    Some(&rx_pipe),
                );
                match self.write_cap(client_id, data_out) {
                    Err(err) => Err(err),
                    Ok(..) => Ok(OctopipesCapMessage::Subscription),
                }
            }
        }
    }

    /// ### manage_unsubscription
    ///
    /// `manage_unsubscription` Handle an unsubscription request stopping the worker associated to this client
    fn manage_unsubscription(
        &mut self,
        client_id: &String,
    ) -> Result<OctopipesCapMessage, OctopipesServerError> {
        //Check if client is already subsribed
        let mut client_exists: bool = false;
        for worker in &self.workers {
            if *client_id == worker.client_id {
                client_exists = true;
                break;
            }
        }
        //If client doesn't exist return error, don't send anything back to client though
        if !client_exists {
            return Err(OctopipesServerError::WorkerNotFound);
        }
        //Stop worker
        match self.stop_worker(client_id) {
            Ok(..) => Ok(OctopipesCapMessage::Unsubscription),
            Err(err) => Err(err),
        }
    }

    /// ### process_first
    ///
    /// `process_first` Find the first Worker which has an available message to process and dispatch it
    /// When OK is returned, the number of processed workers is returned
    /// If no worker has a message to process, the function will just return Ok
    /// If an error was returned during the process, the function will return Error((client_id, Error))
    pub fn process_first(&self) -> Result<usize, (String, OctopipesServerError)> {
        //Iterate over workers
        let mut workers_processed: usize = 0;
        for worker in self.workers.iter() {
            //Get next message
            match worker.get_next_message() {
                Ok(message_opt) => {
                    match message_opt {
                        None => {
                            //If it hasn't any message, just keep iterating
                            continue;
                        }
                        Some(message) => {
                            //If a message is returned, dispatch the message to endpoints
                            if let Err((_, error)) = self.dispatch_message(&message) {
                                return Err((worker.client_id.clone(), error));
                            }
                            //Eventually increment workers processed
                            workers_processed += 1;
                            break; //Break cause we process only the first
                        }
                    }
                }
                Err(error) => {
                    //If an error is returned, return error pairing it with the worker id
                    return Err((worker.client_id.clone(), error));
                }
            }
        }
        Ok(workers_processed)
    }

    /// ### process_once
    ///
    /// `process_once` For each worker process the first message in its inbox. If the worker has no message it will be just ignored.
    /// When OK is returned, the number of processed workers is returned
    /// If no worker has a message to process, the function will just return Ok
    /// If an error was returned during the process, the function will return Error((client_id, Error))
    pub fn process_once(&self) -> Result<usize, (String, OctopipesServerError)> {
        //Iterate over workers
        let mut workers_processed: usize = 0;
        for worker in self.workers.iter() {
            //Get next message
            match worker.get_next_message() {
                Ok(message_opt) => {
                    match message_opt {
                        None => {
                            //If it hasn't any message, just keep iterating
                            continue;
                        }
                        Some(message) => {
                            //If a message is returned, dispatch the message to endpoints
                            if let Err((_, error)) = self.dispatch_message(&message) {
                                return Err((worker.client_id.clone(), error));
                            }
                            //Eventually increment workers processed
                            workers_processed += 1;
                        }
                    }
                }
                Err(error) => {
                    //If an error is returned, return error pairing it with the worker id
                    return Err((worker.client_id.clone(), error));
                }
            }
        }
        Ok(workers_processed)
    }

    /// ### process_once
    ///
    /// `process_once` For each worker process the first message in its inbox. If the worker has no message it will be just ignored.
    /// Once all the workers have been process the function will restart until all the workers has no more message in their inbox.
    /// When OK is returned, the number of processed workers is returned
    /// If no worker has a message to process, the function will just return Ok
    /// If an error was returned during the process, the function will return Error((client_id, Error))
    pub fn process_all(&self) -> Result<usize, (String, OctopipesServerError)> {
        let mut total_workers_processed = 0;
        loop {
            match self.process_once() {
                Ok(workers_processed) => {
                    if workers_processed == 0 {
                        break;
                    }
                    total_workers_processed += workers_processed;
                }
                Err((client, error)) => return Err((client, error)),
            }
        }
        Ok(total_workers_processed)
    }

    //@! Getters
    /// ### is_subscribed
    ///
    /// `is_subscribed` returns whether a client with a certain ID is subscribed or not
    pub fn is_subscribed(&self, client: String) -> Option<std::time::Instant> {
        //Check if a client is subscribed and if it is, return the subscription time
        for worker in &self.workers {
            if worker.client_id == client {
                return Some(worker.subscription.subscription_time);
            }
        }
        None
    }

    /// ### get_subscriptions
    ///
    /// `get_subscriptions` Get all the subscriptions for a certain client
    pub fn get_subscriptions(&self, client: String) -> Option<Vec<String>> {
        for worker in &self.workers {
            if worker.client_id == client {
                //Return groups
                return Some(worker.subscription.groups.clone());
            }
        }
        None
    }

    /// ### get_clients
    ///
    /// `get_clients` Get all the clients id subscribed to the server
    pub fn get_clients(&self) -> Vec<String> {
        let mut clients: Vec<String> = Vec::with_capacity(self.workers.len());
        for worker in &self.workers {
            clients.push(worker.client_id.clone());
        }
        clients
    }

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
        if let Err(..) = pipes::pipe_create(&cli_pipe_rx) {
            return Err(OctopipesServerError::OpenFailed);
        }
        if let Err(..) = pipes::pipe_create(&cli_pipe_tx) {
            return Err(OctopipesServerError::OpenFailed);
        }
        //Prepare thread stuff
        let pipe_read: String = cli_pipe_tx.clone();
        let worker_active: Arc<Mutex<bool>> = Arc::new(Mutex::new(true)); //True
        let thread_active: Arc<Mutex<bool>> = Arc::clone(&worker_active); //Clone active for thread
                                                                          //Create channel
        let (worker_sender, worker_receiver) = mpsc::channel();
        //Start thread
        let join_handle = thread::spawn(move || {
            let mut terminate_thread: bool = false;
            while !terminate_thread {
                //Check if thread has to be stopped
                {
                    let active = thread_active.lock().unwrap();
                    if *active == false {
                        terminate_thread = true;
                    }
                }
                //Try to read from pipe
                match pipes::pipe_read(&pipe_read, 500) {
                    Ok(data) => {
                        //Try to decode data
                        match serializer::decode_message(data) {
                            Ok(message) => {
                                //Send message
                                if let Err(..) = worker_sender.send(Ok(message)) {
                                    terminate_thread = true; //Terminate threda if it wasn't possible to send message to the main thread
                                }
                            }
                            Err(err) => {
                                //Send error
                                if let Err(..) = worker_sender.send(Err(err.to_server_error())) {
                                    terminate_thread = true; //Terminate thread if it wasn't possible to send message to the main thread
                                }
                            }
                        }
                    }
                    Err(..) => {
                        //Send error
                        if let Err(..) = worker_sender.send(Err(OctopipesServerError::ReadFailed)) {
                            terminate_thread = true; //Terminate thread if it wasn't possible to send message to the main thread
                        }
                    }
                }
                thread::sleep(Duration::from_millis(100));
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
            Err(err) => Err(err.to_server_error()),
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

    /// ### get_next_message
    ///
    /// `get_next_message` Get the next available message
    pub fn get_next_message(&self) -> Result<Option<OctopipesMessage>, OctopipesServerError> {
        //Call try recv
        match self.receiver.try_recv() {
            Ok(received) => {
                match received {
                    Ok(message) => Ok(Some(message)), //If a message has been read, return message
                    Err(error) => Err(error),         //Otherwise return error
                }
            }
            Err(recv_error) => {
                match recv_error {
                    mpsc::TryRecvError::Empty => Ok(None), //If recv queue is empty return none
                    _ => Err(OctopipesServerError::WorkerNotRunning), //Otherwise return Worker not running
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
