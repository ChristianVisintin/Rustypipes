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

/// ### decode_subscribe
///
/// `decode_subscribe` decode a subscribe message
fn decode_subscribe(data: &Vec<u8>) -> Result<Vec<String>, OctopipesError> {
    //Size must be at least 2
    if data.len() < 2 {
        return Err(OctopipesError::BadPacket)
    }
    //Check byte 0
    if data[0] != OctopipesCapMessage::Subscribe as u8 {
        return Err(OctopipesError::BadPacket)
    }
    //Get groups amount
    let groups_amount: usize = data[1] as usize;
    //Instance groups
    let mut groups: Vec<String> = Vec::with_capacity(groups_amount);
    //Parse groups
    let mut index: usize = 3;
    while index < data.len() && groups.len() < groups_amount {
        let group_size: usize = data[index] as usize;
        index += 1;
        let final_index: usize = index + group_size;
        let mut group_str: String = String::with_capacity(group_size);
        for byte in &data[index..final_index] {
            group_str.push(*byte as char);
        }
        groups.push(group_str);
        index += final_index;
    }
    if groups.len() != groups_amount {
        return Err(OctopipesError::BadPacket)
    }
    Ok(groups)
}

/// ### decode_assignment
///
/// `decode_assignment` decode an assignment message
fn decode_assignment(data: &Vec<u8>) -> Result<(OctopipesCapError, Option<String>, Option<String>), OctopipesError> {
    //Size must be at least 2
    if data.len() < 2 {
        return Err(OctopipesError::BadPacket)
    }
    //Check byte 0
    if data[0] != OctopipesCapMessage::Assignment as u8 {
        return Err(OctopipesError::BadPacket)
    }
    //Check Cap Error
    let cap_error: OctopipesCapError = OctopipesCapError::from_u8(data[1]);
    //If error is set, don't parse pipes paths
    if cap_error != OctopipesCapError::NoError {
        return Ok((cap_error, None, None))
    }
    //Length must be at least 4 then (will be longer actually)
    if data.len() < 4 {
        return Err(OctopipesError::BadPacket)
    }
    //Otherwise get pipes paths
    let pipe_tx_length: usize = data[2] as usize;
    let minimum_size: usize = 4 + pipe_tx_length;
    //Check new minimum size
    if data.len() < minimum_size {
        return Err(OctopipesError::BadPacket)
    }
    //Get pipe tx
    let index: usize = 3;
    let final_index: usize = index + pipe_tx_length;
    if final_index > data.len() {
        return Err(OctopipesError::BadPacket)
    }
    let mut pipe_tx: String = String::with_capacity(pipe_tx_length);
    for byte in &data[index..final_index] {
        pipe_tx.push(*byte as char);
    }
    let index: usize = final_index;
    if index >= data.len() {
        return Err(OctopipesError::BadPacket)
    }
    //Get pipe rx length
    let pipe_rx_length: usize = data[index] as usize;
    let index: usize = index + 1;
    let final_index: usize = index + pipe_rx_length;
    if final_index > data.len() {
        return Err(OctopipesError::BadPacket)
    }
    let mut pipe_rx: String = String::with_capacity(pipe_rx_length);
    for byte in &data[index..final_index] {
        pipe_rx.push(*byte as char);
    }
    Ok((cap_error, Some(pipe_tx), Some(pipe_rx)))
}

/// ### decode_unsubscribe
///
/// `decode_unsubscribe` decode an unsubscribe message
fn decode_unsubscribe(data: &Vec<u8>) -> Result<OctopipesCapMessage, OctopipesError> {
    //Size must be at least 1
    if data.len() < 1 {
        return Err(OctopipesError::BadPacket)
    }
    //Check byte 0
    if data[0] != OctopipesCapMessage::Unsubscribe as u8 {
        return Err(OctopipesError::BadPacket)
    }
    Ok(OctopipesCapMessage::Unsubscribe)
}
