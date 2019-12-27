//! ## CAP
//!
//! `cap` is the module which takes care of encoding and decoding CAP messages

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
use super::OctopipesError;

/// ### encode_subscribe
///
/// `encode_subscribe` encodes a payload for a SUBSCRIBE CAP message
fn encode_subscribe(groups: Vec<String>) -> Vec<u8> {
    let mut payload_size: usize = 2; //Minimum size
    for group in &groups {
        payload_size += group.len() + 1; //Group len + byte for group len
    }
    //Allocate result
    let mut payload: Vec<u8> = Vec::with_capacity(payload_size);
    payload[0] = OctopipesCapMessage::Subscribe as u8;
    //Group amount
    payload[1] = groups.len() as u8;
    //Iterate over groups
    let mut index: usize = 2;
    for group in &groups {
        //Write group size and then group
        payload[index] = group.len() as u8;
        index += 1;
        //Write group bytes
        for byte in group.as_bytes() {
            payload[index] = *byte;
            index += 1;
        }
    }
    //Return payload when encoded
    payload
}

/// ### encode_assignment
///
/// `encode_assignment` encodes a payload for an ASSIGNMENT CAP message
fn encode_assignment(
    error: OctopipesCapError,
    tx_pipe: Option<String>,
    rx_pipe: Option<String>,
) -> Vec<u8> {
    //Calculate size
    if tx_pipe.is_some() && rx_pipe.is_some() {
        let tx_pipe_str: String = tx_pipe.unwrap().clone();
        let rx_pipe_str: String = rx_pipe.unwrap().clone();
        let payload_size: usize = 2 + tx_pipe_str.len() + rx_pipe_str.len();
        //Allocate result
        let mut payload: Vec<u8> = Vec::with_capacity(payload_size);
        payload[0] = OctopipesCapMessage::Subscribe as u8;
        //Group amount
        payload[1] = error as u8;
        //Write tx pipe len
        payload[2] = tx_pipe_str.len() as u8;
        //Write tx pipe
        let mut index: usize = 3;
        for byte in tx_pipe_str.as_bytes() {
            payload[index] = *byte;
            index += 1;
        }
        //Write rx pipe len
        payload[index] = rx_pipe_str.len() as u8;
        index += 1;
        //Write rx pipe
        for byte in rx_pipe_str.as_bytes() {
            payload[index] = *byte;
            index += 1;
        }
        //Return payload when encoded
        payload
    } else {
        let payload_size: usize = 2;
        let mut payload: Vec<u8> = Vec::with_capacity(payload_size);
        payload[0] = OctopipesCapMessage::Subscribe as u8;
        //Group amount
        payload[1] = error as u8;
        payload
    }
}

/// ### encode_unsubscribe
///
/// `encode_unsubscribe` encodes a payload for an UNSUBSCRIBE CAP message
fn encode_unsubscribe() -> Vec<u8> {
    //Return payload
    vec![OctopipesCapMessage::Unsubscribe as u8]
}

/// ### get_cap_message_type
///
/// `get_cap_message_type` get the message type for a CAP message
fn get_cap_message_type(data: &Vec<u8>) -> Result<OctopipesCapMessage, OctopipesError> {
    if data.len() == 0 {
        return Err(OctopipesError::BadPacket);
    }
    let message_type: OctopipesCapMessage = OctopipesCapMessage::from_u8(data[0]);
    match message_type {
        OctopipesCapMessage::Unknown => Err(OctopipesError::BadPacket),
        _ => Ok(message_type),
    }
}
