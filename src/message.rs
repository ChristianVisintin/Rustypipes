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
}
