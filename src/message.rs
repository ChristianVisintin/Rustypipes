//! ## Message
//!
//! `message` is the module which takes care of managing an OctopipesMessage

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

use crate::OctopipesMessage;
use crate::OctopipesOptions;
use crate::OctopipesProtocolVersion;

impl OctopipesMessage {
    /// ### OctopipesMessage Constructor
    ///
    /// `new` is constructor for OctopipesMessage
    pub(crate) fn new(version: &OctopipesProtocolVersion, origin: &Option<String>, remote: &Option<String>, ttl: u8, options: OctopipesOptions, data: Vec<u8>) -> OctopipesMessage {
        OctopipesMessage {
            version: *version,
            origin: match origin {
                Some(o) => {
                    Some(o.clone())
                },
                None => None
            },
            remote: match remote {
                Some(r) => {
                    Some(r.clone())
                },
                None => None
            },
            ttl: ttl,
            options: options,
            data: data
        }
    }

    /// ### isset_option
    ///
    /// `isset_option` returns wheter an Octopipes Option is set for the current message
    pub fn isset_option(&self, option: OctopipesOptions) -> bool {
        self.options.intersects(option)
    }

    /// ### get_version
    ///
    /// `get_version` returns the message version
    pub fn get_version(&self) -> OctopipesProtocolVersion {
        self.version
    }

    /// ### get_ttl
    ///
    /// `get_ttl` returns the message TTL
    pub fn get_ttl(&self) -> u8 {
        self.ttl
    }

    /// ### get_origin
    ///
    /// `get_origin` returns the message origin
    pub fn get_origin(&self) -> Option<String> {
        match &self.origin {
            Some(origin) => Some(origin.clone()),
            None => None
        }
    }

    /// ### get_remote
    ///
    /// `get_remote` returns the message remote
    pub fn get_remote(&self) -> Option<String> {
        match &self.remote {
            Some(remote) => Some(remote.clone()),
            None => None
        }
    }

    /// ### get_data
    ///
    /// `get_data` returns a reference to data
    pub fn get_data(&self) -> &Vec<u8> {
        &self.data
    }
}

//@! Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message() {
        //Instantiate a message
        let origin: String = String::from("foo");
        let remote: String = String::from("bar");
        let data: Vec<u8> = vec![0x00, 0x01, 0x02, 0x03, 0x04];
        let message = OctopipesMessage::new(&OctopipesProtocolVersion::Version1, &Some(origin.clone()), &Some(remote.clone()), 60, OctopipesOptions::ACK, data);
        assert_eq!(message.get_origin().unwrap(), origin);
        assert_eq!(message.get_remote().unwrap(), remote);
        assert_eq!(message.get_version(), OctopipesProtocolVersion::Version1);
        assert_eq!(message.get_ttl(), 60);
        let msg_data: &Vec<u8> = message.get_data();
        assert_eq!(msg_data[0], 0x00);
        assert_eq!(msg_data[1], 0x01);
        assert_eq!(msg_data[2], 0x02);
        assert_eq!(msg_data[3], 0x03);
        assert_eq!(msg_data[4], 0x04);
    }
}
