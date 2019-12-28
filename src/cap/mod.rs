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
    payload.push(OctopipesCapMessage::Subscribe as u8);
    //Group amount
    payload.push(groups.len() as u8);
    //Iterate over groups
    for group in &groups {
        //Write group size and then group
        payload.push(group.len() as u8);
        //Write group bytes
        for byte in group.as_bytes() {
            payload.push(*byte);
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
        payload.push(OctopipesCapMessage::Assignment as u8);
        //Group amount
        payload.push(error as u8);
        //Write tx pipe len
        payload.push(tx_pipe_str.len() as u8);
        //Write tx pipe
        for byte in tx_pipe_str.as_bytes() {
            payload.push(*byte);
        }
        //Write rx pipe len
        payload.push(rx_pipe_str.len() as u8);
        //Write rx pipe
        for byte in rx_pipe_str.as_bytes() {
            payload.push(*byte);
        }
        //Return payload when encoded
        payload
    } else {
        let payload_size: usize = 2;
        let mut payload: Vec<u8> = Vec::with_capacity(payload_size);
        payload.push(OctopipesCapMessage::Assignment as u8);
        //Group amount
        payload.push(error as u8);
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
    let message_type_opt: Option<OctopipesCapMessage> = OctopipesCapMessage::from_u8(data[0]);
    if message_type_opt.is_none() {
        return Err(OctopipesError::BadPacket);
    }
    match message_type_opt.unwrap() {
        _ => Ok(message_type_opt.unwrap()),
    }
}

/// ### decode_subscribe
///
/// `decode_subscribe` decode a subscribe message
fn decode_subscribe(data: &Vec<u8>) -> Result<Vec<String>, OctopipesError> {
    //Size must be at least 2
    if data.len() < 2 {
        return Err(OctopipesError::BadPacket);
    }
    //Check byte 0
    if data[0] != OctopipesCapMessage::Subscribe as u8 {
        return Err(OctopipesError::BadPacket);
    }
    //Get groups amount
    let groups_amount: usize = data[1] as usize;
    //Instance groups
    let mut groups: Vec<String> = Vec::with_capacity(groups_amount);
    //Parse groups
    let mut index: usize = 2;
    while index < data.len() && groups.len() < groups_amount {
        let group_size: usize = data[index] as usize;
        index += 1;
        let final_index: usize = index + group_size;
        let mut group_str: String = String::with_capacity(group_size);
        for byte in &data[index..final_index] {
            group_str.push(*byte as char);
        }
        groups.push(group_str);
        index = final_index;
    }
    if groups.len() != groups_amount {
        return Err(OctopipesError::BadPacket);
    }
    Ok(groups)
}

/// ### decode_assignment
///
/// `decode_assignment` decode an assignment message
fn decode_assignment(
    data: &Vec<u8>,
) -> Result<(OctopipesCapError, Option<String>, Option<String>), OctopipesError> {
    //Size must be at least 2
    if data.len() < 2 {
        return Err(OctopipesError::BadPacket);
    }
    //Check byte 0
    if data[0] != OctopipesCapMessage::Assignment as u8 {
        return Err(OctopipesError::BadPacket);
    }
    //Check Cap Error
    let cap_error_opt: Option<OctopipesCapError> = OctopipesCapError::from_u8(data[1]);
    if cap_error_opt.is_none() {
        return Err(OctopipesError::BadPacket);
    }
    let cap_error: OctopipesCapError = cap_error_opt.unwrap();
    //If error is set, don't parse pipes paths
    if cap_error != OctopipesCapError::NoError {
        return Ok((cap_error, None, None));
    }
    //Length must be at least 4 then (will be longer actually)
    if data.len() < 4 {
        return Err(OctopipesError::BadPacket);
    }
    //Otherwise get pipes paths
    let pipe_tx_length: usize = data[2] as usize;
    let minimum_size: usize = 4 + pipe_tx_length;
    //Check new minimum size
    if data.len() < minimum_size {
        return Err(OctopipesError::BadPacket);
    }
    //Get pipe tx
    let index: usize = 3;
    let final_index: usize = index + pipe_tx_length;
    if final_index > data.len() {
        return Err(OctopipesError::BadPacket);
    }
    let mut pipe_tx: String = String::with_capacity(pipe_tx_length);
    for byte in &data[index..final_index] {
        pipe_tx.push(*byte as char);
    }
    let index: usize = final_index;
    if index >= data.len() {
        return Err(OctopipesError::BadPacket);
    }
    //Get pipe rx length
    let pipe_rx_length: usize = data[index] as usize;
    let index: usize = index + 1;
    let final_index: usize = index + pipe_rx_length;
    if final_index > data.len() {
        return Err(OctopipesError::BadPacket);
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
        return Err(OctopipesError::BadPacket);
    }
    //Check byte 0
    if data[0] != OctopipesCapMessage::Unsubscribe as u8 {
        return Err(OctopipesError::BadPacket);
    }
    Ok(OctopipesCapMessage::Unsubscribe)
}

//@! Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_subscribe_with_groups() {
        //Test subscribe payload encoding
        //We'll use two groups 'SUBSCRIBE' and 'SYSTEM'
        let payload: Vec<u8> =
            encode_subscribe(vec![String::from("SUBSCRIBE"), String::from("SYSTEM")]);
        assert_eq!(
            payload.len(),
            19,
            "Payload len should be 19, but is {}",
            payload.len()
        );
        //Verify payload data
        assert_eq!(
            payload[0],
            OctopipesCapMessage::Subscribe as u8,
            "Payload at 0 should be {} but is {}",
            OctopipesCapMessage::Subscribe as u8,
            payload[0]
        );
        //Check group amount (2)
        assert_eq!(
            payload[1], 0x02,
            "Payload at 1 should be {} but is {}",
            0x02, payload[1]
        );
        //Group 1 size (9)
        assert_eq!(
            payload[2], 0x09,
            "Payload at 2 should be {} but is {}",
            0x09, payload[2]
        );
        //Group 1 content
        assert_eq!(
            payload[3] as char, 'S',
            "Payload at 3 should be {} but is {}",
            'S', payload[3] as char
        );
        assert_eq!(
            payload[4] as char, 'U',
            "Payload at 4 should be {} but is {}",
            'U', payload[4] as char
        );
        assert_eq!(
            payload[5] as char, 'B',
            "Payload at 5 should be {} but is {}",
            'B', payload[5] as char
        );
        assert_eq!(
            payload[6] as char, 'S',
            "Payload at 6 should be {} but is {}",
            'S', payload[6] as char
        );
        assert_eq!(
            payload[7] as char, 'C',
            "Payload at 7 should be {} but is {}",
            'C', payload[7] as char
        );
        assert_eq!(
            payload[8] as char, 'R',
            "Payload at 8 should be {} but is {}",
            'R', payload[8] as char
        );
        assert_eq!(
            payload[9] as char, 'I',
            "Payload at 9 should be {} but is {}",
            'I', payload[9] as char
        );
        assert_eq!(
            payload[10] as char, 'B',
            "Payload at 10 should be {} but is {}",
            'B', payload[10] as char
        );
        assert_eq!(
            payload[11] as char, 'E',
            "Payload at 11 should be {} but is {}",
            'E', payload[11] as char
        );
        //Group 2 size
        assert_eq!(
            payload[12], 0x06,
            "Payload at 12 should be {} but is {}",
            0x06, payload[12]
        );
        //Groups 2 content
        assert_eq!(
            payload[13] as char, 'S',
            "Payload at 13 should be {} but is {}",
            'S', payload[13] as char
        );
        assert_eq!(
            payload[14] as char, 'Y',
            "Payload at 14 should be {} but is {}",
            'Y', payload[14] as char
        );
        assert_eq!(
            payload[15] as char, 'S',
            "Payload at 15 should be {} but is {}",
            'S', payload[15] as char
        );
        assert_eq!(
            payload[16] as char, 'T',
            "Payload at 16 should be {} but is {}",
            'T', payload[16] as char
        );
        assert_eq!(
            payload[17] as char, 'E',
            "Payload at 17 should be {} but is {}",
            'E', payload[17] as char
        );
        assert_eq!(
            payload[18] as char, 'M',
            "Payload at 18 should be {} but is {}",
            'M', payload[18] as char
        );
    }

    #[test]
    fn test_encode_subscribe_without_groups() {
        //Test subscribe payload encoding
        let payload: Vec<u8> = encode_subscribe(vec![]);
        assert_eq!(
            payload.len(),
            2,
            "Payload len should be 2, but is {}",
            payload.len()
        );
        //Verify payload data
        assert_eq!(
            payload[0],
            OctopipesCapMessage::Subscribe as u8,
            "Payload at 0 should be {} but is {}",
            OctopipesCapMessage::Subscribe as u8,
            payload[0]
        );
        //Check group amount (0)
        assert_eq!(
            payload[1], 0x00,
            "Payload at 1 should be {} but is {}",
            0x00, payload[1]
        );
    }

    #[test]
    fn test_encode_assignment_with_pipes() {
        //Test assignment payload encoding
        //We'll use TX pipe /tmp/pipe_tx and RX pipe /tmp/pipe_rx
        let payload: Vec<u8> = encode_assignment(
            OctopipesCapError::NoError,
            Some(String::from("/tmp/pipe_tx")),
            Some(String::from("/tmp/pipe_rx")),
        );
        assert_eq!(
            payload.len(),
            28,
            "Payload len should be 28, but is {}",
            payload.len()
        );
        //Verify payload data
        assert_eq!(
            payload[0],
            OctopipesCapMessage::Assignment as u8,
            "Payload at 0 should be {} but is {}",
            OctopipesCapMessage::Assignment as u8,
            payload[0]
        );
        //Check CAP error (0)
        assert_eq!(
            payload[1], 0x00,
            "Payload at 1 should be {} but is {}",
            0x00, payload[1]
        );
        //Pipe tx size (12)
        assert_eq!(
            payload[2], 0x0c,
            "Payload at 2 should be {} but is {}",
            0x0c, payload[2]
        );
        //pipe tx content
        assert_eq!(
            payload[3] as char, '/',
            "Payload at 3 should be {} but is {}",
            '/', payload[3] as char
        );
        assert_eq!(
            payload[4] as char, 't',
            "Payload at 4 should be {} but is {}",
            't', payload[4] as char
        );
        assert_eq!(
            payload[5] as char, 'm',
            "Payload at 5 should be {} but is {}",
            'm', payload[5] as char
        );
        assert_eq!(
            payload[6] as char, 'p',
            "Payload at 6 should be {} but is {}",
            'p', payload[6] as char
        );
        assert_eq!(
            payload[7] as char, '/',
            "Payload at 7 should be {} but is {}",
            '/', payload[7] as char
        );
        assert_eq!(
            payload[8] as char, 'p',
            "Payload at 8 should be {} but is {}",
            'p', payload[8] as char
        );
        assert_eq!(
            payload[9] as char, 'i',
            "Payload at 9 should be {} but is {}",
            'i', payload[9] as char
        );
        assert_eq!(
            payload[10] as char, 'p',
            "Payload at 10 should be {} but is {}",
            'p', payload[10] as char
        );
        assert_eq!(
            payload[11] as char, 'e',
            "Payload at 11 should be {} but is {}",
            'e', payload[11] as char
        );
        assert_eq!(
            payload[12] as char, '_',
            "Payload at 12 should be {} but is {}",
            '_', payload[12] as char
        );
        assert_eq!(
            payload[13] as char, 't',
            "Payload at 13 should be {} but is {}",
            't', payload[13] as char
        );
        assert_eq!(
            payload[14] as char, 'x',
            "Payload at 14 should be {} but is {}",
            'x', payload[14] as char
        );
        //pipe rx size (12)
        assert_eq!(
            payload[15], 0x0c,
            "Payload at 12 should be {} but is {}",
            0x0c, payload[15]
        );
        //Pipe rx content
        assert_eq!(
            payload[16] as char, '/',
            "Payload at 3 should be {} but is {}",
            '/', payload[16] as char
        );
        assert_eq!(
            payload[17] as char, 't',
            "Payload at 4 should be {} but is {}",
            't', payload[17] as char
        );
        assert_eq!(
            payload[18] as char, 'm',
            "Payload at 5 should be {} but is {}",
            'm', payload[18] as char
        );
        assert_eq!(
            payload[19] as char, 'p',
            "Payload at 6 should be {} but is {}",
            'p', payload[19] as char
        );
        assert_eq!(
            payload[20] as char, '/',
            "Payload at 7 should be {} but is {}",
            '/', payload[20] as char
        );
        assert_eq!(
            payload[21] as char, 'p',
            "Payload at 8 should be {} but is {}",
            'p', payload[21] as char
        );
        assert_eq!(
            payload[22] as char, 'i',
            "Payload at 9 should be {} but is {}",
            'i', payload[22] as char
        );
        assert_eq!(
            payload[23] as char, 'p',
            "Payload at 10 should be {} but is {}",
            'p', payload[23] as char
        );
        assert_eq!(
            payload[24] as char, 'e',
            "Payload at 11 should be {} but is {}",
            'e', payload[24] as char
        );
        assert_eq!(
            payload[25] as char, '_',
            "Payload at 12 should be {} but is {}",
            '_', payload[25] as char
        );
        assert_eq!(
            payload[26] as char, 'r',
            "Payload at 13 should be {} but is {}",
            'r', payload[26] as char
        );
        assert_eq!(
            payload[27] as char, 'x',
            "Payload at 14 should be {} but is {}",
            'x', payload[27] as char
        );
    }

    #[test]
    fn test_encode_assignment_with_error() {
        //Test assignment payload encoding
        let payload: Vec<u8> = encode_assignment(OctopipesCapError::NameAlreadyTaken, None, None);
        assert_eq!(
            payload.len(),
            2,
            "Payload len should be 2, but is {}",
            payload.len()
        );
        //Verify payload data
        assert_eq!(
            payload[0],
            OctopipesCapMessage::Assignment as u8,
            "Payload at 0 should be {} but is {}",
            OctopipesCapMessage::Assignment as u8,
            payload[0]
        );
        //Check CAP error (NameAlreadyTaken => 0x01)
        assert_eq!(
            payload[1], 0x01,
            "Payload at 1 should be {} but is {}",
            0x01, payload[1]
        );
    }

    #[test]
    fn test_encode_unsubscribe() {
        //Test unsubscribe payload encoding
        let payload: Vec<u8> = encode_unsubscribe();
        assert_eq!(
            payload.len(),
            1,
            "Payload len should be 1, but is {}",
            payload.len()
        );
        //Verify payload data
        assert_eq!(
            payload[0],
            OctopipesCapMessage::Unsubscribe as u8,
            "Payload at 0 should be {} but is {}",
            OctopipesCapMessage::Unsubscribe as u8,
            payload[0]
        );
    }

    #[test]
    fn test_get_cap_message_type() {
        //Test subscribe
        let payload: Vec<u8> = encode_subscribe(vec![String::from("SUBSCRIBE")]);
        assert_eq!(
            get_cap_message_type(&payload).unwrap(),
            OctopipesCapMessage::Subscribe,
            "Subscribe message has wrong cap type: {}",
            get_cap_message_type(&payload).unwrap()
        );
        //Test assignment
        let payload: Vec<u8> = encode_assignment(
            OctopipesCapError::NoError,
            Some(String::from("/tmp/pipe_tx")),
            Some(String::from("/tmp/pipe_rx")),
        );
        assert_eq!(
            get_cap_message_type(&payload).unwrap(),
            OctopipesCapMessage::Assignment,
            "Assignment message has wrong cap type: {}",
            get_cap_message_type(&payload).unwrap()
        );
        //Test Unsubscribe
        let payload: Vec<u8> = encode_unsubscribe();
        assert_eq!(
            get_cap_message_type(&payload).unwrap(),
            OctopipesCapMessage::Unsubscribe,
            "Unsubscribe message has wrong cap type: {}",
            get_cap_message_type(&payload).unwrap()
        );
    }

    #[test]
    fn test_parse_subscribe() {
        //Create a subscribe payload to decode (two groups, SYS and HW)
        let payload: Vec<u8> = vec![0x01, 0x02, 0x03, 'S' as u8, 'Y' as u8, 'S' as u8, 0x02, 'H' as u8, 'W' as u8];
        match decode_subscribe(&payload) {
            Ok(groups) => {
                //Check groups
                assert_eq!(groups.len(), 2, "There should be two groups; found {}", groups.len());
                //Check group 0
                assert_eq!(groups[0], "SYS", "First group should be 'SYS' but is {}", groups[0]);
                //Check group 1
                assert_eq!(groups[1], "HW", "Second group should be 'HW' but is {}", groups[1]);
            },
            Err(error) => {
                panic!("Subscribe parsing failed: {}", error);
            }
        }
    }

    #[test]
    fn test_parse_subscribe_nok() {
        //Create a subscribe payload to decode (two groups, but one is missing)
        let payload: Vec<u8> = vec![0x01, 0x02, 0x03, 'S' as u8, 'Y' as u8, 'S' as u8];
        match decode_subscribe(&payload) {
            Ok(..) => {
                panic!("Decode should have failed");
            },
            Err(error) => {
                assert_eq!(error, OctopipesError::BadPacket, "Subscribe NOK should have returned BAD PACKET, but returned {}", error);
            }
        }
    }

    #[test]
    fn test_parse_assignment() {
        //Create an assignment payload to decode (two pipes /tmp/pipe_tx, /tmp/pipe_rx)
        let payload: Vec<u8> = vec![0xff, 0x00, 0x0c, 0x2f, 0x74, 0x6d, 0x70, 0x2f, 0x70, 0x69, 0x70, 0x65, 0x5f, 0x74, 0x78, 0x0c, 0x2f, 0x74, 0x6d, 0x70, 0x2f, 0x70, 0x69, 0x70, 0x65, 0x5f, 0x72, 0x78];
        match decode_assignment(&payload) {
            Ok((cap_error, pipe_tx, pipe_rx)) => {
                //Cap Error should be none
                assert_eq!(cap_error, OctopipesCapError::NoError, "CAP Error should be NoError, but is {}", cap_error);
                //Pipe tx
                assert!(pipe_tx.is_some(), "Pipe TX shouldn't be None");
                let pipe_tx_str = pipe_tx.unwrap();
                assert_eq!(pipe_tx_str, String::from("/tmp/pipe_tx"), "Pipe tx should be /tmp/pipe_tx, but is {}", pipe_tx_str);
                //Pipe rx
                assert!(pipe_rx.is_some(), "Pipe RX shouldn't be None");
                let pipe_rx_str = pipe_rx.unwrap();
                assert_eq!(pipe_rx_str, String::from("/tmp/pipe_rx"), "Pipe rx should be /tmp/pipe_rx, but is {}", pipe_rx_str);
            },
            Err(..) => panic!("Assignment decode shouldn't have returned error")
        }
    }
    //TODO: parse assignment with CAP error
    //TODO: parse assignment error
    //TODO: parse unsubscribe
    //TODO: parse unsubscribe error
}
