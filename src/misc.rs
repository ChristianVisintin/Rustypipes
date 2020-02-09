//! # Misc
//!
//! `misc` contains different implementations for octopipes types

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
use crate::OctopipesError;
use crate::OctopipesProtocolVersion;
use crate::OctopipesOptions;
use crate::OctopipesServerError;

use std::fmt;

//Types utils
impl OctopipesProtocolVersion {
    pub(crate) fn from_u8(value: u8) -> Option<OctopipesProtocolVersion> {
        match value {
            1 => Some(OctopipesProtocolVersion::Version1),
            _ => None,
        }
    }
}

impl OctopipesCapMessage {
    pub(crate) fn from_u8(value: u8) -> Option<OctopipesCapMessage> {
        match value {
            0x01 => Some(OctopipesCapMessage::Subscription),
            0x02 => Some(OctopipesCapMessage::Unsubscription),
            0xff => Some(OctopipesCapMessage::Assignment),
            _ => None,
        }
    }
    pub(crate) fn to_string(&self) -> &str {
        match self {
            OctopipesCapMessage::Assignment => "ASSIGNMENT",
            OctopipesCapMessage::Subscription => "SUBSCRIPTION",
            OctopipesCapMessage::Unsubscription => "UNSUBSCRIPTION"
        }
    }
}

impl OctopipesCapError {
    pub(crate) fn from_u8(value: u8) -> Option<OctopipesCapError> {
        match value {
            0x00 => Some(OctopipesCapError::NoError),
            0x01 => Some(OctopipesCapError::NameAlreadyTaken),
            0x02 => Some(OctopipesCapError::FileSystemError),
            _ => None
        }
    }
    pub(crate) fn to_string(&self) -> &str {
        match self {
            OctopipesCapError::FileSystemError => "FileSystemError",
            OctopipesCapError::NameAlreadyTaken => "NameAlreadyTaken",
            OctopipesCapError::NoError => "NoError"
        }
    }
}

impl OctopipesOptions {
    pub(crate) fn from_u8(value: u8) -> OctopipesOptions {
        let mut option: OctopipesOptions = OctopipesOptions::empty();
        if value & OctopipesOptions::RCK.bits() != 0 {
            option.set(OctopipesOptions::RCK, true);
        }
        if value & OctopipesOptions::ACK.bits() != 0 {
            option.set(OctopipesOptions::ACK, true);
        }
        if value & OctopipesOptions::ICK.bits() != 0 {
            option.set(OctopipesOptions::ICK, true);
        }
        option
    }
}

impl OctopipesError {
    pub fn to_string(&self) -> &str {
        match self {
            OctopipesError::Uninitialized => "OctopipesClient is not initialized yet",
            OctopipesError::BadChecksum => "Packet has bad checksum",
            OctopipesError::BadPacket => "It was not possible to decode packet, since it contains bad data",
            OctopipesError::CapTimeout => "CAP timeout",
            OctopipesError::NoDataAvailable => "There is not data available on pipe",
            OctopipesError::NotSubscribed => "The client must be subscribed to the server before receiving and sending messages",
            OctopipesError::NotUnsubscribed => "The client must be unsubscribed to perform this action",
            OctopipesError::OpenFailed => "Could not open the requested pipe",
            OctopipesError::ReadFailed => "Could not read from pipe",
            OctopipesError::ThreadAlreadyRunning => "Client loop Thread is already running",
            OctopipesError::ThreadError => "Thread error",
            OctopipesError::UnsupportedVersion => "Unsupported protocol version",
            OctopipesError::WriteFailed => "Could not write to pipe",
            _ => "Unknown error"
        }
    }
    pub fn to_server_error(&self) -> OctopipesServerError {
        match self {
            OctopipesError::BadChecksum => OctopipesServerError::BadChecksum,
            OctopipesError::BadPacket => OctopipesServerError::BadPacket,
            OctopipesError::CapTimeout => OctopipesServerError::CapTimeout,
            OctopipesError::OpenFailed => OctopipesServerError::OpenFailed,
            OctopipesError::ReadFailed => OctopipesServerError::ReadFailed,
            OctopipesError::WriteFailed => OctopipesServerError::WriteFailed,
            OctopipesError::UnsupportedVersion => OctopipesServerError::UnsupportedVersion,
            _ => OctopipesServerError::Unknown
        }
    }
}

impl OctopipesServerError {
    pub fn to_string(&self) -> &str {
        match self {
            OctopipesServerError::Uninitialized => "OctopipesServer is not initialized yet",
            OctopipesServerError::BadChecksum => "Packet has bad checksum",
            OctopipesServerError::BadClientDir => "Could not create client directory",
            OctopipesServerError::BadPacket => "It was not possible to decode packet, since it contains bad data",
            OctopipesServerError::CapTimeout => "CAP timeout",
            OctopipesServerError::NoRecipient => "The provided message has no recipient",
            OctopipesServerError::OpenFailed => "Could not open the requested pipe",
            OctopipesServerError::ReadFailed => "Could not read from pipe",
            OctopipesServerError::ThreadAlreadyRunning => "Server CAP loop Thread is already running",
            OctopipesServerError::ThreadError => "Thread error",
            OctopipesServerError::WorkerAlreadyRunning => "The requested worker is already running",
            OctopipesServerError::WorkerNotFound => "The requested Worker couldn't be found",
            OctopipesServerError::WorkerExists => "The requested Worker already exists",
            OctopipesServerError::WorkerNotRunning => "This worker is not running",
            OctopipesServerError::WriteFailed => "Could not write to pipe",
            OctopipesServerError::UnsupportedVersion => "Unsupported protocol version",
            _ => "Unknown error"
        }
    }
}

impl fmt::Debug for OctopipesError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl fmt::Display for OctopipesError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl fmt::Debug for OctopipesServerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl fmt::Display for OctopipesServerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl fmt::Display for OctopipesOptions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.bits())
    }
}

impl fmt::Display for OctopipesCapMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl fmt::Debug for OctopipesCapMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl fmt::Display for OctopipesCapError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl fmt::Debug for OctopipesCapError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_version() {
        let protocol_version: Option<OctopipesProtocolVersion> = OctopipesProtocolVersion::from_u8(0x01);
        assert_eq!(protocol_version.unwrap(), OctopipesProtocolVersion::Version1);
        let protocol_version: Option<OctopipesProtocolVersion> = OctopipesProtocolVersion::from_u8(0xf0);
        assert!(protocol_version.is_none());
    }

    #[test]
    fn test_cap_message() {
        let cap_message: Option<OctopipesCapMessage> = OctopipesCapMessage::from_u8(0x01);
        assert_eq!(cap_message.unwrap(), OctopipesCapMessage::Subscription);
        assert_eq!(cap_message.unwrap().to_string(), String::from("SUBSCRIPTION"));
        let cap_message: Option<OctopipesCapMessage> = OctopipesCapMessage::from_u8(0x02);
        assert_eq!(cap_message.unwrap(), OctopipesCapMessage::Unsubscription);
        assert_eq!(cap_message.unwrap().to_string(), String::from("UNSUBSCRIPTION"));
        let cap_message: Option<OctopipesCapMessage> = OctopipesCapMessage::from_u8(0xff);
        assert_eq!(cap_message.unwrap(), OctopipesCapMessage::Assignment);
        assert_eq!(cap_message.unwrap().to_string(), String::from("ASSIGNMENT"));
        println!("DEBUG: {:?}", cap_message);
        let cap_message: Option<OctopipesCapMessage> = OctopipesCapMessage::from_u8(0xe0);
        assert!(cap_message.is_none());
    }

    #[test]
    fn test_cap_error() {
        let cap_error: Option<OctopipesCapError> = OctopipesCapError::from_u8(0x00);
        assert_eq!(cap_error.unwrap(), OctopipesCapError::NoError);
        assert_eq!(cap_error.unwrap().to_string(), String::from("NoError"));
        let cap_error: Option<OctopipesCapError> = OctopipesCapError::from_u8(0x01);
        assert_eq!(cap_error.unwrap(), OctopipesCapError::NameAlreadyTaken);
        assert_eq!(cap_error.unwrap().to_string(), String::from("NameAlreadyTaken"));
        let cap_error: Option<OctopipesCapError> = OctopipesCapError::from_u8(0x02);
        assert_eq!(cap_error.unwrap(), OctopipesCapError::FileSystemError);
        assert_eq!(cap_error.unwrap().to_string(), String::from("FileSystemError"));
        println!("DEBUG: {:?}", cap_error);
        let cap_error: Option<OctopipesCapError> = OctopipesCapError::from_u8(0xe0);
        assert!(cap_error.is_none());
    }

    #[test]
    fn test_message_option() {
        let message_options: OctopipesOptions = OctopipesOptions::from_u8(0x07); //7
        assert!(message_options.intersects(OctopipesOptions::RCK));
        assert!(message_options.intersects(OctopipesOptions::ACK));
        assert!(message_options.intersects(OctopipesOptions::ICK));
        let message_options: OctopipesOptions = OctopipesOptions::from_u8(0x03); //3
        assert!(message_options.intersects(OctopipesOptions::RCK));
        assert!(message_options.intersects(OctopipesOptions::ACK));
        assert!(!message_options.intersects(OctopipesOptions::ICK));
        let message_options: OctopipesOptions = OctopipesOptions::from_u8(0x01); //3
        assert!(message_options.intersects(OctopipesOptions::RCK));
        assert!(!message_options.intersects(OctopipesOptions::ACK));
        assert!(!message_options.intersects(OctopipesOptions::ICK));
        let message_options: OctopipesOptions = OctopipesOptions::from_u8(0x04); //4
        assert!(!message_options.intersects(OctopipesOptions::RCK));
        assert!(!message_options.intersects(OctopipesOptions::ACK));
        assert!(message_options.intersects(OctopipesOptions::ICK));
        let message_options: OctopipesOptions = OctopipesOptions::from_u8(0x05); //5
        assert!(message_options.intersects(OctopipesOptions::RCK));
        assert!(!message_options.intersects(OctopipesOptions::ACK));
        assert!(message_options.intersects(OctopipesOptions::ICK));
        //Print options
        println!("{}", message_options);
    }

    #[test]
    fn test_octopipes_error() {
        let octopipes_error: OctopipesError = OctopipesError::Uninitialized;
        assert_eq!(octopipes_error.to_string(), String::from("OctopipesClient is not initialized yet"));
        assert_eq!(octopipes_error.to_server_error(), OctopipesServerError::Unknown);
        let octopipes_error: OctopipesError = OctopipesError::BadChecksum;
        assert_eq!(octopipes_error.to_string(), String::from("Packet has bad checksum"));
        assert_eq!(octopipes_error.to_server_error(), OctopipesServerError::BadChecksum);
        let octopipes_error: OctopipesError = OctopipesError::BadPacket;
        assert_eq!(octopipes_error.to_string(), String::from("It was not possible to decode packet, since it contains bad data"));
        assert_eq!(octopipes_error.to_server_error(), OctopipesServerError::BadPacket);
        let octopipes_error: OctopipesError = OctopipesError::CapTimeout;
        assert_eq!(octopipes_error.to_string(), String::from("CAP timeout"));
        assert_eq!(octopipes_error.to_server_error(), OctopipesServerError::CapTimeout);
        let octopipes_error: OctopipesError = OctopipesError::NoDataAvailable;
        assert_eq!(octopipes_error.to_string(), String::from("There is not data available on pipe"));
        assert_eq!(octopipes_error.to_server_error(), OctopipesServerError::Unknown);
        let octopipes_error: OctopipesError = OctopipesError::NotSubscribed;
        assert_eq!(octopipes_error.to_string(), String::from("The client must be subscribed to the server before receiving and sending messages"));
        assert_eq!(octopipes_error.to_server_error(), OctopipesServerError::Unknown);
        let octopipes_error: OctopipesError = OctopipesError::NotUnsubscribed;
        assert_eq!(octopipes_error.to_string(), String::from("The client must be unsubscribed to perform this action"));
        assert_eq!(octopipes_error.to_server_error(), OctopipesServerError::Unknown);
        let octopipes_error: OctopipesError = OctopipesError::OpenFailed;
        assert_eq!(octopipes_error.to_string(), String::from("Could not open the requested pipe"));
        assert_eq!(octopipes_error.to_server_error(), OctopipesServerError::OpenFailed);
        let octopipes_error: OctopipesError = OctopipesError::ReadFailed;
        assert_eq!(octopipes_error.to_string(), String::from("Could not read from pipe"));
        assert_eq!(octopipes_error.to_server_error(), OctopipesServerError::ReadFailed);
        let octopipes_error: OctopipesError = OctopipesError::ThreadAlreadyRunning;
        assert_eq!(octopipes_error.to_string(), String::from("Client loop Thread is already running"));
        assert_eq!(octopipes_error.to_server_error(), OctopipesServerError::Unknown);
        let octopipes_error: OctopipesError = OctopipesError::ThreadError;
        assert_eq!(octopipes_error.to_string(), String::from("Thread error"));
        assert_eq!(octopipes_error.to_server_error(), OctopipesServerError::Unknown);
        let octopipes_error: OctopipesError = OctopipesError::UnsupportedVersion;
        assert_eq!(octopipes_error.to_string(), String::from("Unsupported protocol version"));
        assert_eq!(octopipes_error.to_server_error(), OctopipesServerError::UnsupportedVersion);
        let octopipes_error: OctopipesError = OctopipesError::WriteFailed;
        assert_eq!(octopipes_error.to_string(), String::from("Could not write to pipe"));
        assert_eq!(octopipes_error.to_server_error(), OctopipesServerError::WriteFailed);
        let octopipes_error: OctopipesError = OctopipesError::Unknown;
        assert_eq!(octopipes_error.to_string(), String::from("Unknown error"));
        assert_eq!(octopipes_error.to_server_error(), OctopipesServerError::Unknown);
        println!("{:?}", octopipes_error);
    }

    #[test]
    fn test_octopipes_server_error() {
        let octopipes_error: OctopipesServerError = OctopipesServerError::Uninitialized;
        assert_eq!(octopipes_error.to_string(), String::from("OctopipesServer is not initialized yet"));
        let octopipes_error: OctopipesServerError = OctopipesServerError::BadChecksum;
        assert_eq!(octopipes_error.to_string(), String::from("Packet has bad checksum"));
        let octopipes_error: OctopipesServerError = OctopipesServerError::BadClientDir;
        assert_eq!(octopipes_error.to_string(), String::from("Could not create client directory"));
        let octopipes_error: OctopipesServerError = OctopipesServerError::BadPacket;
        assert_eq!(octopipes_error.to_string(), String::from("It was not possible to decode packet, since it contains bad data"));
        let octopipes_error: OctopipesServerError = OctopipesServerError::CapTimeout;
        assert_eq!(octopipes_error.to_string(), String::from("CAP timeout"));
        let octopipes_error: OctopipesServerError = OctopipesServerError::NoRecipient;
        assert_eq!(octopipes_error.to_string(), String::from("The provided message has no recipient"));
        let octopipes_error: OctopipesServerError = OctopipesServerError::OpenFailed;
        assert_eq!(octopipes_error.to_string(), String::from("Could not open the requested pipe"));
        let octopipes_error: OctopipesServerError = OctopipesServerError::ReadFailed;
        assert_eq!(octopipes_error.to_string(), String::from("Could not read from pipe"));
        let octopipes_error: OctopipesServerError = OctopipesServerError::ThreadAlreadyRunning;
        assert_eq!(octopipes_error.to_string(), String::from("Server CAP loop Thread is already running"));
        let octopipes_error: OctopipesServerError = OctopipesServerError::ThreadError;
        assert_eq!(octopipes_error.to_string(), String::from("Thread error"));
        let octopipes_error: OctopipesServerError = OctopipesServerError::WorkerAlreadyRunning;
        assert_eq!(octopipes_error.to_string(), String::from("The requested worker is already running"));
        let octopipes_error: OctopipesServerError = OctopipesServerError::WorkerExists;
        assert_eq!(octopipes_error.to_string(), String::from("The requested Worker already exists"));
        let octopipes_error: OctopipesServerError = OctopipesServerError::WorkerNotFound;
        assert_eq!(octopipes_error.to_string(), String::from("The requested Worker couldn't be found"));
        let octopipes_error: OctopipesServerError = OctopipesServerError::WorkerNotRunning;
        assert_eq!(octopipes_error.to_string(), String::from("This worker is not running"));
        let octopipes_error: OctopipesServerError = OctopipesServerError::WriteFailed;
        assert_eq!(octopipes_error.to_string(), String::from("Could not write to pipe"));
        let octopipes_error: OctopipesServerError = OctopipesServerError::UnsupportedVersion;
        assert_eq!(octopipes_error.to_string(), String::from("Unsupported protocol version"));
        let octopipes_error: OctopipesServerError = OctopipesServerError::Unknown;
        assert_eq!(octopipes_error.to_string(), String::from("Unknown error"));
        println!("DEBUG: {:?}; DISPLAY: {}", octopipes_error, octopipes_error);
    }
}
