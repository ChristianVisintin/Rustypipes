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

use super::OctopipesError;
use super::OctopipesProtocolVersion;
use super::OctopipesOptions;
use super::OctopipesState;

use std::fmt;

//Types utils
impl OctopipesProtocolVersion {
    pub fn from_u8(value: u8) -> OctopipesProtocolVersion {
        match value {
            1 => OctopipesProtocolVersion::Version1,
            _ => OctopipesProtocolVersion::Unknown,
        }
    }
}

impl OctopipesOptions {
    pub fn from_u8(value: u8) -> OctopipesOptions {
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

impl fmt::Display for OctopipesOptions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.bits())
    }
}

impl OctopipesError {
    pub fn to_string(&self) -> &str {
        match self {
            OctopipesError::Success => "Not an error",
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
