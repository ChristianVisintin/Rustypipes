//! # RustyPipes
//!
//! `rusty-pipes` is a library which provides functions to work with the Octopipes Protocol

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

mod cap;
pub mod client;
pub mod message;
pub(crate) mod misc;
mod pipes;
mod serializer;
pub mod server;

use std::thread;

#[macro_use]
extern crate bitflags;

pub const RUSTYPIPES_VERSION: &str = "0.1.0";
pub const RUSTYPIPES_VERSION_MAJOR: i32 = 0;
pub const RUSTYPIPES_VERSION_MINOR: i32 = 1;

/// ## Data types

/// ### OctopipesError
///
/// `OctopipesError` describes the kind of error returned by an operation on the OctopipesClient

#[derive(Copy, Clone, PartialEq)]
pub enum OctopipesError {
    Uninitialized,
    BadPacket,
    BadChecksum,
    UnsupportedVersion,
    NoDataAvailable,
    OpenFailed,
    WriteFailed,
    ReadFailed,
    CapTimeout,
    NotSubscribed,
    NotUnsubscribed,
    ThreadError,
    ThreadAlreadyRunning,
    Unknown,
}

/// ### OctopipesCapError
///
/// `OctopipesCapError` describes the kind of error returned by an operation on the CAP
#[derive(Copy, Clone, PartialEq)]
pub enum OctopipesCapError {
    NoError = 0,
    NameAlreadyTaken = 1,
    FileSystemError = 2,
}

/// ### OctopipesCapMessage
///
/// `OctopipesCapMessage` describes the CAP message type

#[derive(Copy, Clone, PartialEq)]
pub enum OctopipesCapMessage {
    Subscription = 0x01,
    Unsubscription = 0x02,
    Assignment = 0xff,
}

/// ### OctopipesState
///
/// `OctopipesState` describes the current state of the OctopipesClient

#[derive(Copy, Clone, PartialEq)]
pub enum OctopipesState {
    Initialized,
    Subscribed,
    Running,
    Unsubscribed,
    Stopped,
}

bitflags! {

    /// ### OctopipesState
    ///
    /// `OctopipesState` describes the current state of the OctopipesClient
    pub struct OctopipesOptions: u8 {
        const RCK = 0b00000001;
        const ACK = 0b00000010;
        const ICK = 0b00000100;
    }
}

/// ### OctopipesProtocolVersion
///
/// `OctopipesProtocolVersion` describes the protocol version used by the client

#[derive(Copy, Clone)]
pub enum OctopipesProtocolVersion {
    Version1 = 1,
}

/// ### OctopipesMessage
///
/// `OctopipesMessage` contains the data of a message

pub struct OctopipesMessage {
    version: OctopipesProtocolVersion,
    origin: Option<String>,
    remote: Option<String>,
    ttl: u8,
    options: OctopipesOptions,
    checksum: u8,
    data: Vec<u8>,
}

/// ### OctopipesClient
///
/// `OctopipesClient` is a container for an Octopipes Client

pub struct OctopipesClient {
    //Client params
    id: String,
    version: OctopipesProtocolVersion,
    //Pipes paths
    cap_pipe: String,
    tx_pipe: Option<String>,
    rx_pipe: Option<String>,
    //State
    state: OctopipesState,
    //Thread
    client_loop: Option<thread::JoinHandle<()>>,
    //Callbacks
    on_received_fn: Option<fn(&OctopipesClient, Result<&OctopipesMessage, &OctopipesError>)>,
    on_sent_fn: Option<fn(&OctopipesClient, &OctopipesMessage)>,
    on_subscribed_fn: Option<fn(&OctopipesClient)>,
    on_unsubscribed_fn: Option<fn(&OctopipesClient)>,
}

/// ### OctopipesServer
///
/// `OctopipesServer` is a container for an Octopipes Server
pub struct OctopipesServer {
    //Server params
    version: OctopipesProtocolVersion,
    //Subscription list
    //Pipe
    cap_pipe: String,
    //Thread
    cap_listener: Option<thread::JoinHandle<()>>,
    //workers
    workers: Vec<OctopipesServerWorker>,
    //Callbacks
    on_cap_message_fn: fn(Result<&OctopipesMessage, &OctopipesError>),
    on_receive: fn(Result<&OctopipesMessage, &OctopipesError>),
}

/// ### OctopipesServerWorker
///
/// `OctopipesServerWorker` is a container for an Octopipes Server worker (a client handler)
struct OctopipesServerWorker {
    //Associated client
    client_id: String,
    subscription: Subscription,
    //Pipes
    pipe_read: String,  //TX Pipe of the client
    pipe_write: String, //RX Pipe of the client
    //Join handle
    worker_loop: thread::JoinHandle<()>,
}

/// ### Subscription
///
/// `Subscription` is a struct which stores the data for a single subscription from a client
struct Subscription {
    subscription_time: std::time::SystemTime,
    groups: Vec<String>,
}
