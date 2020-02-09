//! ## Serializer
//!
//! `serializer` is the module which takes care of encoding and decoding the Octopipes Messages

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

use crate::OctopipesError;
use crate::OctopipesMessage;
use crate::OctopipesOptions;
use crate::OctopipesProtocolVersion;

const SOH: u8 = 0x01;
const STX: u8 = 0x02;
const ETX: u8 = 0x03;

const MINIMUM_SIZE_VERSION_1: usize = 17;

/// ### encode_message
///
/// `encode_message` encodes an OctopipesMessage struct to an Octopipes packet
pub(super) fn encode_message(message: &OctopipesMessage) -> Result<Vec<u8>, OctopipesError> {
    //Match version
    match message.version {
        OctopipesProtocolVersion::Version1 => {
            //Start with calculating the data size
            let mut data_size = MINIMUM_SIZE_VERSION_1; //Minimum size
            //Sum message origin
            match &message.origin {
                Some(origin) => {
                    data_size = data_size + origin.len();
                }
                None => {}
            }
            //Sum message remote
            match &message.remote {
                Some(remote) => {
                    data_size = data_size + remote.len();
                }
                None => {}
            }
            data_size = data_size + message.data.len();
            //Initialize data
            let mut data_out: Vec<u8> = Vec::with_capacity(data_size);
            //Encode data
            //Start of header
            data_out.push(SOH);
            //Version
            data_out.push(message.version as u8);
            //Origin
            match &message.origin {
                Some(origin) => {
                    data_out.push(origin.len() as u8);
                    //Push origin bytes to Vec
                    for byte in origin.as_bytes() {
                        data_out.push(*byte);
                    }
                }
                None => {
                    //Push 0x00 which is origin size
                    data_out.push(0x00);
                }
            }
            //Remote
            match &message.remote {
                Some(remote) => {
                    data_out.push(remote.len() as u8);
                    //Push remote bytes to vec
                    for byte in remote.as_bytes() {
                        data_out.push(*byte);
                    }
                }
                None => {
                    //Push 0x00 which is remote size
                    data_out.push(0x00);
                }
            }
            //TTL
            data_out.push(message.ttl);
            //Data Size
            let payload_size_64: u64 = message.data.len() as u64;
            for i in (0..8).rev() {
                let val: u8 = ((payload_size_64 >> (i * 8)) & 0xFF) as u8;
                data_out.push(val);
            }
            //Options
            data_out.push(message.options.bits());
            //Track checksum index
            let checksum_index = data_out.len();
            data_out.push(0x00);
            //STX
            data_out.push(STX);
            //Write data
            for byte in &message.data {
                data_out.push(*byte);
            }
            //ETX
            data_out.push(ETX);
            //if isset option IGNORE CHECKSUM do not set checksum
            if !message.isset_option(OctopipesOptions::ICK) {
                //set checksum
                let checksum = calculate_checksum(message);
                data_out[checksum_index] = checksum;
            }
            Ok(data_out)
        }
    }
}

/// ### decode_message
///
/// `decode_message` decodes a message in bytes to an OctopipesMessage struct
pub(super) fn decode_message(data: Vec<u8>) -> Result<OctopipesMessage, OctopipesError> {
    let current_min_size = 2; //SOH, version
    if data.len() < current_min_size {
        return Err(OctopipesError::BadPacket);
    }
    //Check SOH
    if *data.get(0).unwrap() != SOH {
        return Err(OctopipesError::BadPacket);
    }
    let version_opt: Option<OctopipesProtocolVersion> = OctopipesProtocolVersion::from_u8(*data.get(1).unwrap());
    match version_opt {
        Some(version) => {
            match version {
                OctopipesProtocolVersion::Version1 => {
                    let mut current_min_size: usize = MINIMUM_SIZE_VERSION_1; //Minimum packet size
                    let mut curr_index: usize = 2;
                    let mut final_index: usize;
                    if data.len() < current_min_size {
                        return Err(OctopipesError::BadPacket);
                    }
                    //Get sizes
                    let origin_size: usize = *data.get(2).unwrap() as usize;
                    current_min_size = current_min_size + origin_size;
                    if data.len() < current_min_size {
                        return Err(OctopipesError::BadPacket);
                    }
                    //Get origin
                    curr_index += 1;
                    final_index = curr_index + origin_size;
                    let origin: Option<String>;
                    if origin_size > 0 {
                        let mut origin_str: String = String::with_capacity(origin_size);
                        for byte in &data[curr_index..final_index] {
                            origin_str.push(*byte as char);
                        }
                        origin = Some(origin_str);
                    } else {
                        origin = None
                    }
                    curr_index = final_index;
                    let remote_size: usize = *data.get(curr_index).unwrap() as usize;
                    current_min_size += remote_size;
                    curr_index += 1;
                    final_index = curr_index + remote_size;
                    if data.len() < current_min_size {
                        return Err(OctopipesError::BadPacket);
                    }
                    let remote: Option<String>;
                    if remote_size > 0 {
                        let mut remote_str: String = String::with_capacity(remote_size);
                        for byte in &data[curr_index..final_index] {
                            remote_str.push(*byte as char);
                        }
                        remote = Some(remote_str);
                    } else {
                        remote = None
                    }
                    curr_index = final_index;
                    //TTL
                    let ttl: u8 = *data.get(curr_index).unwrap();
                    curr_index += 1;
                    //Data Size
                    let mut data_size: u64 = 0;
                    let mut final_index = curr_index + 8;
                    for byte in &data[curr_index..final_index] {
                        data_size = data_size << 8;
                        data_size += *byte as u64;
                    }
                    curr_index = final_index;
                    //Options
                    let options: OctopipesOptions =
                        OctopipesOptions::from_u8(*data.get(curr_index).unwrap());
                    curr_index += 1;
                    //Checksum
                    let checksum: u8 = *data.get(curr_index).unwrap();
                    curr_index += 1;
                    //STX
                    if *data.get(curr_index).unwrap() != STX {
                        return Err(OctopipesError::BadPacket);
                    }
                    curr_index += 1;
                    //Data
                    final_index = curr_index + (data_size as usize);
                    //Verify if data fits
                    if final_index >= data.len() {
                        return Err(OctopipesError::BadPacket);
                    }
                    if *data.get(final_index).unwrap() != ETX {
                        //Check if last byte is ETX
                        return Err(OctopipesError::BadPacket);
                    }
                    let mut payload = Vec::with_capacity(data_size as usize);
                    for byte in &data[curr_index..final_index] {
                        payload.push(*byte);
                    }
                    //Instance OctopipesMessage
                    let message: OctopipesMessage = OctopipesMessage::new(
                        &version, &origin, &remote, ttl, options, payload,
                    );
                    //Verify checksum if required
                    if !message.isset_option(OctopipesOptions::ICK) {
                        if checksum != calculate_checksum(&message) {
                            return Err(OctopipesError::BadChecksum);
                        }
                    }
                    Ok(message)
                }
            }
        }
        None => Err(OctopipesError::UnsupportedVersion),
    }
}

/// ### calculate_checksum
///
/// `calculate_checksum` Calculate checksum for the provided Octopipes Message
fn calculate_checksum(message: &OctopipesMessage) -> u8 {
    let mut checksum: u8 = SOH;
    match message.version {
        OctopipesProtocolVersion::Version1 => {
            checksum = checksum ^ (message.version as u8);
            match &message.origin {
                Some(origin) => {
                    checksum = checksum ^ (origin.len() as u8);
                    for byte in origin.as_bytes() {
                        checksum = checksum ^ byte;
                    }
                }
                None => checksum = checksum ^ 0x00,
            }
            match &message.remote {
                Some(remote) => {
                    checksum = checksum ^ (remote.len() as u8);
                    for byte in remote.as_bytes() {
                        checksum = checksum ^ byte;
                    }
                }
                None => checksum = checksum ^ 0x00,
            }
            checksum = checksum ^ message.ttl;
            //Data Size
            let payload_size_64: u64 = message.data.len() as u64;
            for i in (0..8).rev() {
                let val: u8 = ((payload_size_64 >> (i * 8)) & 0xFF) as u8;
                checksum = checksum ^ val;
            }
            //Options
            checksum = checksum ^ (message.options.bits());
            //Checksum with STX
            checksum = checksum ^ STX;
            //Checksum with data
            for byte in &message.data {
                checksum = checksum ^ *byte;
            }
            //Checksum with ETX
            checksum = checksum ^ ETX;
        }
    }
    checksum
}

//@! Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_rck() {
        println!("Testing encode with RCK");
        let payload: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
        let origin: String = String::from("test_client");
        let remote: String = String::from("test_remote");
        //Calculate estimated size
        let predicted_size: usize =
            MINIMUM_SIZE_VERSION_1 + origin.len() + remote.len() + payload.len();
        //Prepare message
        let message: OctopipesMessage = OctopipesMessage::new(
            &OctopipesProtocolVersion::Version1,
            &Some(origin.clone()),
            &Some(remote.clone()),
            60,
            OctopipesOptions::RCK,
            payload,
        );
        //Encode message
        let data: Vec<u8> = encode_message(&message).expect("Could not encode message");
        let checksum = calculate_checksum(&message);
        //Dump data
        print!("Data dump: ");
        for byte in &data {
            print!("{:02x} ", *byte);
        }
        println!("");
        //Check if size is correct
        assert_eq!(
            predicted_size,
            data.len(),
            "Expected size {} is different from data size {}",
            predicted_size,
            data.len()
        );
        println!("Data size is correct: {}", data.len());
        //Verify payload bytes
        println!("Verifying if data bytes are encoded as we expect");
        //SOH
        assert_eq!(
            *data.get(0).unwrap(),
            SOH,
            "Byte at 0: {:02x} is not SOH {:02x}",
            *data.get(0).unwrap(),
            SOH
        );
        //Version
        assert_eq!(
            *data.get(1).unwrap(),
            OctopipesProtocolVersion::Version1 as u8,
            "Byte at 1: {:02x} is not {:02x}",
            *data.get(1).unwrap(),
            OctopipesProtocolVersion::Version1 as u8
        );
        //Origin size
        assert_eq!(
            *data.get(2).unwrap() as usize,
            origin.len(),
            "Byte at 2: {:02x} is not {:02x}",
            *data.get(2).unwrap() as usize,
            origin.len()
        );
        //Origin
        println!("Checking Origin");
        let origin_bytes = &data[3..14];
        let origin_chars: Vec<char> = origin.chars().collect();
        for i in 0..origin.len() {
            assert_eq!(
                origin_chars[i],
                origin_bytes[i] as char,
                "Byte at {}: {} is not {}",
                i + 2,
                origin_chars[i],
                origin_bytes[i] as char
            );
        }
        //Remote size
        assert_eq!(
            *data.get(14).unwrap() as usize,
            remote.len(),
            "Byte at 14: {:02x} is not {:02x}",
            *data.get(14).unwrap() as usize,
            remote.len()
        );
        //Remote
        let remote_bytes = &data[15..26];
        let remote_chars: Vec<char> = remote.chars().collect();
        for i in 0..remote.len() {
            assert_eq!(
                remote_chars[i],
                remote_bytes[i] as char,
                "Byte at {}: {} is not {}",
                i + 15,
                remote_chars[i],
                remote_bytes[i] as char
            );
        }
        //TTL
        assert_eq!(
            *data.get(26).unwrap(),
            60,
            "Byte at 26: {:02x} is not {:02x}",
            *data.get(26).unwrap() as usize,
            60
        );
        //Data Size (should be 9)
        assert_eq!(
            *data.get(27).unwrap(),
            0,
            "Byte at 27: {:02x} is not {:02x}",
            *data.get(27).unwrap(),
            0
        );
        assert_eq!(
            *data.get(28).unwrap(),
            0,
            "Byte at 28: {:02x} is not {:02x}",
            *data.get(28).unwrap(),
            0
        );
        assert_eq!(
            *data.get(29).unwrap(),
            0,
            "Byte at 29: {:02x} is not {:02x}",
            *data.get(29).unwrap(),
            0
        );
        assert_eq!(
            *data.get(30).unwrap(),
            0,
            "Byte at 30: {:02x} is not {:02x}",
            *data.get(30).unwrap(),
            0
        );
        assert_eq!(
            *data.get(31).unwrap(),
            0,
            "Byte at 31: {:02x} is not {:02x}",
            *data.get(31).unwrap(),
            0
        );
        assert_eq!(
            *data.get(32).unwrap(),
            0,
            "Byte at 32: {:02x} is not {:02x}",
            *data.get(32).unwrap(),
            0
        );
        assert_eq!(
            *data.get(33).unwrap(),
            0,
            "Byte at 33: {:02x} is not {:02x}",
            *data.get(33).unwrap(),
            0
        );
        assert_eq!(
            *data.get(34).unwrap(),
            9,
            "Byte at 34: {:02x} is not {:02x}",
            *data.get(34).unwrap(),
            9
        );
        //Options (RCK)
        assert_eq!(
            *data.get(35).unwrap(),
            1,
            "Byte at 35: {:02x} is not {:02x}",
            *data.get(35).unwrap(),
            1
        );
        //Checksum
        assert_eq!(
            *data.get(36).unwrap(),
            checksum,
            "Byte at 36: {:02x} is not {:02x}",
            *data.get(36).unwrap(),
            checksum
        );
        //STX
        assert_eq!(
            *data.get(37).unwrap(),
            STX,
            "Byte at 37: {:02x} is not {:02x}",
            *data.get(37).unwrap(),
            STX
        );
        //Data
        assert_eq!(
            *data.get(38).unwrap(),
            1,
            "Byte at 38: {:02x} is not {:02x}",
            *data.get(38).unwrap(),
            1
        );
        assert_eq!(
            *data.get(39).unwrap(),
            2,
            "Byte at 39: {:02x} is not {:02x}",
            *data.get(39).unwrap(),
            2
        );
        assert_eq!(
            *data.get(40).unwrap(),
            3,
            "Byte at 40: {:02x} is not {:02x}",
            *data.get(40).unwrap(),
            3
        );
        assert_eq!(
            *data.get(41).unwrap(),
            4,
            "Byte at 41: {:02x} is not {:02x}",
            *data.get(41).unwrap(),
            4
        );
        assert_eq!(
            *data.get(42).unwrap(),
            5,
            "Byte at 42: {:02x} is not {:02x}",
            *data.get(42).unwrap(),
            5
        );
        assert_eq!(
            *data.get(43).unwrap(),
            6,
            "Byte at 43: {:02x} is not {:02x}",
            *data.get(43).unwrap(),
            6
        );
        assert_eq!(
            *data.get(44).unwrap(),
            7,
            "Byte at 44: {:02x} is not {:02x}",
            *data.get(44).unwrap(),
            7
        );
        assert_eq!(
            *data.get(45).unwrap(),
            8,
            "Byte at 45: {:02x} is not {:02x}",
            *data.get(45).unwrap(),
            8
        );
        assert_eq!(
            *data.get(46).unwrap(),
            9,
            "Byte at 46: {:02x} is not {:02x}",
            *data.get(46).unwrap(),
            9
        );
        //ETX
        assert_eq!(
            *data.get(47).unwrap(),
            ETX,
            "Byte at 47: {:02x} is not {:02x}",
            *data.get(47).unwrap(),
            ETX
        );
        println!("Packet encoded successfully");
    }

    #[test]
    fn test_encode_ick() {
        println!("Testing encode with ICK and without remote");
        let payload: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
        let origin: String = String::from("test_client");
        //Calculate estimated size
        let predicted_size: usize = MINIMUM_SIZE_VERSION_1 + origin.len() + payload.len();
        //Prepare message
        let message: OctopipesMessage = OctopipesMessage::new(
            &OctopipesProtocolVersion::Version1,
            &Some(origin.clone()),
            &None,
            60,
            OctopipesOptions::ICK,
            payload,
        );
        //Encode message
        let data: Vec<u8> = encode_message(&message).expect("Could not encode message");
        let checksum = 0; //Should be 0
        assert_eq!(
            checksum, 0,
            "Checksum should be 0, but is {:02x}, even if ICK is set",
            checksum
        );
        //Dump data
        print!("Data dump: ");
        for byte in &data {
            print!("{:02x} ", *byte);
        }
        println!("\n");
        //Check if size is correct
        assert_eq!(
            predicted_size,
            data.len(),
            "Expected size {} is different from data size {}",
            predicted_size,
            data.len()
        );
        println!("Data size is correct: {}", data.len());
        //Verify payload bytes
        println!("Verifying if data bytes are encoded as we expect");
        //SOH
        assert_eq!(
            *data.get(0).unwrap(),
            SOH,
            "Byte at 0: {:02x} is not SOH {:02x}",
            *data.get(0).unwrap(),
            SOH
        );
        //Version
        assert_eq!(
            *data.get(1).unwrap(),
            OctopipesProtocolVersion::Version1 as u8,
            "Byte at 1: {:02x} is not {:02x}",
            *data.get(1).unwrap(),
            OctopipesProtocolVersion::Version1 as u8
        );
        //Origin size
        assert_eq!(
            *data.get(2).unwrap() as usize,
            origin.len(),
            "Byte at 2: {:02x} is not {:02x}",
            *data.get(2).unwrap() as usize,
            origin.len()
        );
        //Origin
        println!("Checking Origin");
        let origin_bytes = &data[3..14];
        let origin_chars: Vec<char> = origin.chars().collect();
        for i in 0..origin.len() {
            assert_eq!(
                origin_chars[i],
                origin_bytes[i] as char,
                "Byte at {}: {} is not {}",
                i + 2,
                origin_chars[i],
                origin_bytes[i] as char
            );
        }
        //Remote size
        assert_eq!(
            *data.get(14).unwrap() as usize,
            0,
            "Byte at 14: {:02x} is not {:02x}",
            *data.get(14).unwrap() as usize,
            0
        );
        //TTL
        assert_eq!(
            *data.get(15).unwrap(),
            60,
            "Byte at 26: {:02x} is not {:02x}",
            *data.get(15).unwrap() as usize,
            60
        );
        //Data Size (should be 9)
        assert_eq!(
            *data.get(16).unwrap(),
            0,
            "Byte at 27: {:02x} is not {:02x}",
            *data.get(16).unwrap(),
            0
        );
        assert_eq!(
            *data.get(17).unwrap(),
            0,
            "Byte at 28: {:02x} is not {:02x}",
            *data.get(17).unwrap(),
            0
        );
        assert_eq!(
            *data.get(18).unwrap(),
            0,
            "Byte at 29: {:02x} is not {:02x}",
            *data.get(18).unwrap(),
            0
        );
        assert_eq!(
            *data.get(19).unwrap(),
            0,
            "Byte at 30: {:02x} is not {:02x}",
            *data.get(19).unwrap(),
            0
        );
        assert_eq!(
            *data.get(20).unwrap(),
            0,
            "Byte at 31: {:02x} is not {:02x}",
            *data.get(20).unwrap(),
            0
        );
        assert_eq!(
            *data.get(21).unwrap(),
            0,
            "Byte at 32: {:02x} is not {:02x}",
            *data.get(21).unwrap(),
            0
        );
        assert_eq!(
            *data.get(22).unwrap(),
            0,
            "Byte at 33: {:02x} is not {:02x}",
            *data.get(22).unwrap(),
            0
        );
        assert_eq!(
            *data.get(23).unwrap(),
            9,
            "Byte at 34: {:02x} is not {:02x}",
            *data.get(23).unwrap(),
            9
        );
        //Options (ICK)
        assert_eq!(
            *data.get(24).unwrap(),
            4,
            "Byte at 35: {:02x} is not {:02x}",
            *data.get(24).unwrap(),
            4
        );
        //Checksum
        assert_eq!(
            *data.get(25).unwrap(),
            checksum,
            "Byte at 36: {:02x} is not {:02x}",
            *data.get(25).unwrap(),
            checksum
        );
        //STX
        assert_eq!(
            *data.get(26).unwrap(),
            STX,
            "Byte at 37: {:02x} is not {:02x}",
            *data.get(26).unwrap(),
            STX
        );
        //Data
        assert_eq!(
            *data.get(27).unwrap(),
            1,
            "Byte at 38: {:02x} is not {:02x}",
            *data.get(27).unwrap(),
            1
        );
        assert_eq!(
            *data.get(28).unwrap(),
            2,
            "Byte at 39: {:02x} is not {:02x}",
            *data.get(28).unwrap(),
            2
        );
        assert_eq!(
            *data.get(29).unwrap(),
            3,
            "Byte at 40: {:02x} is not {:02x}",
            *data.get(29).unwrap(),
            3
        );
        assert_eq!(
            *data.get(30).unwrap(),
            4,
            "Byte at 41: {:02x} is not {:02x}",
            *data.get(30).unwrap(),
            4
        );
        assert_eq!(
            *data.get(31).unwrap(),
            5,
            "Byte at 42: {:02x} is not {:02x}",
            *data.get(31).unwrap(),
            5
        );
        assert_eq!(
            *data.get(32).unwrap(),
            6,
            "Byte at 43: {:02x} is not {:02x}",
            *data.get(32).unwrap(),
            6
        );
        assert_eq!(
            *data.get(33).unwrap(),
            7,
            "Byte at 44: {:02x} is not {:02x}",
            *data.get(33).unwrap(),
            7
        );
        assert_eq!(
            *data.get(34).unwrap(),
            8,
            "Byte at 45: {:02x} is not {:02x}",
            *data.get(34).unwrap(),
            8
        );
        assert_eq!(
            *data.get(35).unwrap(),
            9,
            "Byte at 46: {:02x} is not {:02x}",
            *data.get(35).unwrap(),
            9
        );
        //ETX
        assert_eq!(
            *data.get(36).unwrap(),
            ETX,
            "Byte at 47: {:02x} is not {:02x}",
            *data.get(36).unwrap(),
            ETX
        );
        println!("Packet encoded successfully");
    }

    #[test]
    fn test_encode_empty() {
        println!("Testing encode with no origin/remote/payload");
        //Calculate estimated size
        let predicted_size: usize = MINIMUM_SIZE_VERSION_1;
        //Prepare message
        let message: OctopipesMessage = OctopipesMessage::new(
            &OctopipesProtocolVersion::Version1,
            &None,
            &None,
            0,
            OctopipesOptions::empty(),
            vec![],
        );
        //Encode message
        let data: Vec<u8> = encode_message(&message).expect("Could not encode message");
        let checksum = calculate_checksum(&message);
        //Dump data
        print!("Data dump: ");
        for byte in &data {
            print!("{:02x} ", *byte);
        }
        println!("\n");
        //Check if size is correct
        assert_eq!(
            predicted_size,
            data.len(),
            "Expected size {} is different from data size {}",
            predicted_size,
            data.len()
        );
        println!("Data size is correct: {}", data.len());
        //Verify payload bytes
        println!("Verifying if data bytes are encoded as we expect");
        //SOH
        assert_eq!(
            *data.get(0).unwrap(),
            SOH,
            "Byte at 0: {:02x} is not SOH {:02x}",
            *data.get(0).unwrap(),
            SOH
        );
        //Version
        assert_eq!(
            *data.get(1).unwrap(),
            OctopipesProtocolVersion::Version1 as u8,
            "Byte at 1: {:02x} is not {:02x}",
            *data.get(1).unwrap(),
            OctopipesProtocolVersion::Version1 as u8
        );
        //Origin size
        assert_eq!(
            *data.get(2).unwrap() as usize,
            0,
            "Byte at 2: {:02x} is not {:02x}",
            *data.get(2).unwrap(),
            0
        );
        //Remote size
        assert_eq!(
            *data.get(3).unwrap() as usize,
            0,
            "Byte at 3: {:02x} is not {:02x}",
            *data.get(3).unwrap() as usize,
            0
        );
        //TTL
        assert_eq!(
            *data.get(4).unwrap(),
            0,
            "Byte at 4: {:02x} is not {:02x}",
            *data.get(4).unwrap() as usize,
            60
        );
        //Data Size (should be 9)
        assert_eq!(
            *data.get(5).unwrap(),
            0,
            "Byte at 5: {:02x} is not {:02x}",
            *data.get(5).unwrap(),
            0
        );
        assert_eq!(
            *data.get(6).unwrap(),
            0,
            "Byte at 6: {:02x} is not {:02x}",
            *data.get(6).unwrap(),
            0
        );
        assert_eq!(
            *data.get(7).unwrap(),
            0,
            "Byte at 7: {:02x} is not {:02x}",
            *data.get(7).unwrap(),
            0
        );
        assert_eq!(
            *data.get(8).unwrap(),
            0,
            "Byte at 8: {:02x} is not {:02x}",
            *data.get(8).unwrap(),
            0
        );
        assert_eq!(
            *data.get(9).unwrap(),
            0,
            "Byte at 9: {:02x} is not {:02x}",
            *data.get(9).unwrap(),
            0
        );
        assert_eq!(
            *data.get(10).unwrap(),
            0,
            "Byte at 10: {:02x} is not {:02x}",
            *data.get(10).unwrap(),
            0
        );
        assert_eq!(
            *data.get(11).unwrap(),
            0,
            "Byte at 11: {:02x} is not {:02x}",
            *data.get(11).unwrap(),
            0
        );
        assert_eq!(
            *data.get(12).unwrap(),
            0,
            "Byte at 12: {:02x} is not {:02x}",
            *data.get(12).unwrap(),
            0
        );
        //Options (None)
        assert_eq!(
            *data.get(13).unwrap(),
            0,
            "Byte at 13: {:02x} is not {:02x}",
            *data.get(13).unwrap(),
            0
        );
        //Checksum
        assert_eq!(
            *data.get(14).unwrap(),
            checksum,
            "Byte at 14: {:02x} is not {:02x}",
            *data.get(14).unwrap(),
            checksum
        );
        //STX
        assert_eq!(
            *data.get(15).unwrap(),
            STX,
            "Byte at 15: {:02x} is not {:02x}",
            *data.get(15).unwrap(),
            STX
        );
        //ETX
        assert_eq!(
            *data.get(16).unwrap(),
            ETX,
            "Byte at 16: {:02x} is not {:02x}",
            *data.get(16).unwrap(),
            ETX
        );
        println!("Packet encoded successfully");
    }

    #[test]
    fn test_decode_simple() {
        println!("Testing decoding simple");
        //Create a test packet
        let data_in: Vec<u8> = vec![
            0x01, 0x01, 0x0b, 0x74, 0x65, 0x73, 0x74, 0x5f, 0x70, 0x61, 0x72, 0x73, 0x65, 0x72,
            0x09, 0x42, 0x52, 0x4f, 0x41, 0x44, 0x43, 0x41, 0x53, 0x54, 0x3c, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x20, 0x00, 0x0e, 0x02, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05,
            0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13,
            0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x03,
        ];
        //Decode packet and check if parameters are correct
        let message: OctopipesMessage = decode_message(data_in).expect("Could not decode message");
        //Verify origin (test_parser)
        let origin: String = message.origin.unwrap().clone();
        assert_eq!(
            origin, "test_parser",
            "Origin should be 'test_parser', but is {}",
            origin
        );
        //Verify remote (BROADCAST)
        let remote: String = message.remote.unwrap().clone();
        assert_eq!(
            remote, "BROADCAST",
            "Remote should be 'BROADCAST', but is {}",
            remote
        );
        //Options is empty
        assert_eq!(
            message.options,
            OctopipesOptions::empty(),
            "Option should be 0, but is {}",
            message.options
        );
        //TTL is 60
        assert_eq!(message.ttl, 60, "TTL should be 60, but is {}", message.ttl);
        //Data size should be 32
        assert_eq!(
            message.data.len(),
            32,
            "Data size should be 32, but is {}",
            message.data.len()
        );
        //Compare data
        for byte in 0..32 {
            assert_eq!(
                message.data[byte], byte as u8,
                "Data byte {} should be {} but is {}",
                byte, byte, message.data[byte]
            );
        }
        println!("Decode simple passed");
    }

    #[test]
    fn test_decode_unsupported_version() {
        println!("Testing decoding unsupported version");
        //Create a test packet (byte 1 is ff which is version 255, which doesn't exist)
        let data_in: Vec<u8> = vec![
            0x01, 0xff, 0x0b, 0x74, 0x65, 0x73, 0x74, 0x5f, 0x70, 0x61, 0x72, 0x73, 0x65, 0x72,
            0x09, 0x42, 0x52, 0x4f, 0x41, 0x44, 0x43, 0x41, 0x53, 0x54, 0x3c, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x20, 0x00, 0x0e, 0x02, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05,
            0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13,
            0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x03,
        ];
        //Decode packet and check if parameters are correct
        match decode_message(data_in) {
            Ok(..) => {
                panic!("Decoding should have failed");
            }
            Err(error) => match error {
                OctopipesError::UnsupportedVersion => {
                    println!("Successfully returned unsupported version")
                }
                _ => panic!(
                    "Decoding should have returned unsupported version, but returned: {}",
                    error
                ),
            },
        }
        println!("Decode Unsupported Version passed");
    }

    #[test]
    fn test_decode_bad_parts() {
        println!("Testing decoding 1 byte length");
        let data_in: Vec<u8> = vec![
            0x01
        ];
        //Decode packet and check if parameters are correct
        if let Ok(_) = decode_message(data_in) {
            panic!("Decode of 1 byte message should have returned Err");
        }
        println!("Testing decoding with bad SOH");
        let data_in: Vec<u8> = vec![
            0x04, 0x01, 0x0b, 0x74, 0x65, 0x73, 0x74, 0x5f, 0x70, 0x61, 0x72, 0x73, 0x65, 0x72,
            0x09, 0x42, 0x52, 0x4f, 0x41, 0x44, 0x43, 0x41, 0x53, 0x54, 0x3c, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x20, 0x00, 0x0e, 0x02, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05,
            0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13,
            0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x03
        ];
        //Decode packet and check if parameters are correct
        if let Ok(_) = decode_message(data_in) {
            panic!("Decode of bad SOH should have returned error");
        }
        println!("Testing decoding with bad minimum length");
        let data_in: Vec<u8> = vec![
            0x01, 0x01, 0x0b, 0x74
        ];
        //Decode packet and check if parameters are correct
        if let Ok(_) = decode_message(data_in) {
            panic!("Decode of bad length should have returned error");
        }
        println!("Testing decoding with bad minimum origin length");
        let data_in: Vec<u8> = vec![
            0x01, 0x01, 0x0b, 0x74, 0x65, 0x73, 0x74, 0x5f, 0x70, 0x61, 0x72, 0x73, 0x65, 0x72,
            0x09, 0x42, 0x52, 0x4f, 0x41, 0x44, 0x43, 0x41
        ];
        //Decode packet and check if parameters are correct
        if let Ok(_) = decode_message(data_in) {
            panic!("Decode of bad length should have returned error");
        }
        println!("Testing decoding with bad ETX");
        let data_in: Vec<u8> = vec![
            0x04, 0x01, 0x0b, 0x74, 0x65, 0x73, 0x74, 0x5f, 0x70, 0x61, 0x72, 0x73, 0x65, 0x72,
            0x09, 0x42, 0x52, 0x4f, 0x41, 0x44, 0x43, 0x41, 0x53, 0x54, 0x3c, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x20, 0x00, 0x0e, 0x02, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05,
            0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13,
            0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0xf0
        ];
        //Decode packet and check if parameters are correct
        if let Ok(_) = decode_message(data_in) {
            panic!("Decode of bad ETX should have returned error");
        }
    }

    #[test]
    fn test_decode_bad_checksum() {
        println!("Testing decoding bad checksum");
        //Create a test packet (checksum changed from 0x0e => 0x30)
        let data_in: Vec<u8> = vec![
            0x01, 0x01, 0x0b, 0x74, 0x65, 0x73, 0x74, 0x5f, 0x70, 0x61, 0x72, 0x73, 0x65, 0x72,
            0x09, 0x42, 0x52, 0x4f, 0x41, 0x44, 0x43, 0x41, 0x53, 0x54, 0x3c, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x20, 0x00, 0x30, 0x02, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05,
            0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13,
            0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x03,
        ];
        //Decode packet and check if parameters are correct
        match decode_message(data_in) {
            Ok(..) => {
                panic!("Decoding should have failed");
            }
            Err(error) => match error {
                OctopipesError::BadChecksum => println!("Successfully returned bad checksum"),
                _ => panic!(
                    "Decoding should have returned bad checksum, but returned: {}",
                    error
                ),
            },
        }
        println!("Decode Bad Checksum passed");
    }

    #[test]
    fn test_decode_no_origin() {
        println!("Testing decoding no origin");
        //Create a test packet (Look at remote 0x00, I've set ICK, because I didn't want to recalc checksum)
        let data_in: Vec<u8> = vec![
            0x01, 0x01, 0x0b, 0x74, 0x65, 0x73, 0x74, 0x5f, 0x70, 0x61, 0x72, 0x73, 0x65, 0x72,
            0x00, 0x3c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x04, 0x00, 0x02, 0x00,
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
            0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c,
            0x1d, 0x1e, 0x1f, 0x03,
        ];
        //Decode packet and check if parameters are correct
        match decode_message(data_in) {
            Ok(message) => {
                //Check if remote is correct (None)
                match message.remote {
                    Some(remote) => panic!("Remote should be None but is Some {}", remote),
                    None => println!("Remote is None, as expected"),
                }
                //Origin should be correct though
                match message.origin {
                    Some(origin) => {
                        assert_eq!(
                            origin, "test_parser",
                            "remote should be test_parser but is {}",
                            origin
                        );
                    }
                    None => panic!("Origin shouldn't be None"),
                }
            }
            Err(..) => {
                panic!("Decoding shouldn't fail");
            }
        }
        println!("Decode No origin passed");
    }

    #[test]
    fn test_decode_no_remote() {
        println!("Testing decoding no remote");
        //Create a test packet (Look at origin 0x00, I've set ICK, because I didn't want to recalc checksum)
        let data_in: Vec<u8> = vec![
            0x01, 0x01, 0x00, 0x09, 0x42, 0x52, 0x4f, 0x41, 0x44, 0x43, 0x41, 0x53, 0x54, 0x3c,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x04, 0x00, 0x02, 0x00, 0x01, 0x02,
            0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
            0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e,
            0x1f, 0x03,
        ];
        //Decode packet and check if parameters are correct
        match decode_message(data_in) {
            Ok(message) => {
                //Check if origin is correct (None)
                match message.origin {
                    Some(origin) => panic!("Origin should be None but is Some {}", origin),
                    None => println!("Origin is None, as expected"),
                }
                //Remote should be correct though
                match message.remote {
                    Some(remote) => {
                        assert_eq!(
                            remote, "BROADCAST",
                            "remote should be BROADCAST but is {}",
                            remote
                        );
                    }
                    None => panic!("Remote shouldn't be None"),
                }
            }
            Err(..) => {
                panic!("Decoding shouldn't fail");
            }
        }
        println!("Decode No remote passed");
    }

    #[test]
    fn test_decode_bad_encoded() {
        println!("Testing decoding bad encoded");
        //Create a test packet (Look at origin 0x00, I've set ICK, because I didn't want to recalc checksum)
        let data_in: Vec<u8> = vec![
            0x01, 0x01, 0x00, 0x09, 0x42, 0x52, 0x4f, 0x41, 0x44, 0x43, 0x41, 0x53, 0x54, 0x3c,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x04, 0x00, 0x02, 0x00, 0x01, 0x02,
            0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
            0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e,
            0x1f,
        ];
        //Decode packet and check if parameters are correct
        match decode_message(data_in) {
            Ok(..) => panic!("Decoding should have returned error, since ETX is missing"),
            Err(error) => match error {
                OctopipesError::BadPacket => println!("Decoding successfully returned Bad Packet"),
                _ => panic!(
                    "Decoding should have returned bad packet, but returned {}",
                    error
                ),
            },
        }
        println!("Decode Bad encoded passed");
    }
}
