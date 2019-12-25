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

pub mod client;
mod cap;
mod pipes;
mod serializer;

use std::thread;

pub const RUSTYPIPES_VERSION: &str = "0.1.0";
pub const RUSTYPIPES_VERSION_MAJOR: i32 = 0;
pub const RUSTYPIPES_VERSION_MINOR: i32 = 1;

/// ## Data types

/// ### OctopipesError
///
/// `OctopipesError` describes the kind of error returned by an operation on the OctopipesClient

#[derive(Copy, Clone)]
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
    BadAlloc,
    Unknown
}

/// ### OctopipesState
///
/// `OctopipesState` describes the current state of the OctopipesClient

#[derive(Copy, Clone)]
pub enum OctopipesState {
    Initialized,
    Subscribed,
    Running,
    Unsubscribed,
    Stopped
}

/// ### OctopipesState
///
/// `OctopipesState` describes the current state of the OctopipesClient

#[derive(Copy, Clone)]
pub enum OctopipesOptions {
    Null = 0,
    RequireAck = 1,
    Ack = 2,
    IgnoreChecksum = 4
}

/// ### OctopipesProtocolVersion
///
/// `OctopipesProtocolVersion` describes the protocol version used by the client

#[derive(Copy, Clone)]
pub enum OctopipesProtocolVersion {
    Version1 = 1
}

/// ### OctopipesMessage
///
/// `OctopipesMessage` contains the data of a message

pub struct OctopipesMessage {
    version: OctopipesProtocolVersion,
    origin: String,
    remote: String,
    ttl: u8,
    options: OctopipesOptions,
    checksum: u8,
    data_size: u64,
    data: Vec<u64>
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
    on_unsubscribed_fn: Option<fn(&OctopipesClient)>
}
