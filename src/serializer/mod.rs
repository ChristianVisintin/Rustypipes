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

use super::OctopipesError;
use super::OctopipesMessage;
use super::OctopipesOptions;
use super::OctopipesProtocolVersion;

const SOH: u8 = 0x01;
const STX: u8 = 0x02;
const ETX: u8 = 0x03;

const MINIMUM_SIZE_VERSION_1: usize = 17;

/// ### encode_message
///
/// `encode_message` encodes an OctopipesMessage struct to an Octopipes packet
fn encode_message(message: &mut OctopipesMessage) -> Result<Vec<u8>, OctopipesError> {
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
            for i in 7..1 {
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
        _ => Err(OctopipesError::UnsupportedVersion), //Return Unsupported Version
    }
}

/// ### decode_message
///
/// `decode_message` decodes a message in bytes to an OctopipesMessage struct
fn decode_message(data: Vec<u8>) -> Result<OctopipesMessage, OctopipesError> {
    let current_min_size = 2; //SOH, version
    if data.len() < current_min_size {
        return Err(OctopipesError::BadPacket)
    }
    //Check SOH
    if *data.get(0).unwrap() != SOH {
        return Err(OctopipesError::BadPacket)
    }
    let version: OctopipesProtocolVersion = OctopipesProtocolVersion::from_u8(*data.get(1).unwrap());
    match version {
        OctopipesProtocolVersion::Version1 => {
            let mut current_min_size: usize = MINIMUM_SIZE_VERSION_1; //Minimum packet size
            let mut curr_index: usize = 2;
            let mut final_index: usize = 0;
            if data.len() < current_min_size {
                return Err(OctopipesError::BadPacket)
            }
            //Get sizes
            let origin_size: usize = *data.get(2).unwrap() as usize;
            current_min_size = current_min_size + origin_size;
            if data.len() < current_min_size {
                return Err(OctopipesError::BadPacket)
            }
            //Get origin
            curr_index += 1;
            final_index = curr_index + origin_size;
            let mut origin:  Option<String>;
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
                return Err(OctopipesError::BadPacket)
            }
            let mut remote: Option<String>;
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
                data_size += *byte as u64;
                data_size = data_size << 8;
            }
            curr_index = final_index;
            //Options
            let options: OctopipesOptions = OctopipesOptions::from_u8(*data.get(curr_index).unwrap());
            curr_index += 1;
            //Checksum
            let checksum: u8 = *data.get(curr_index).unwrap();
            curr_index += 1;
            //STX
            if *data.get(curr_index).unwrap() != STX {
                return Err(OctopipesError::BadPacket)
            }
            //Data
            final_index = curr_index + (data_size as usize);
            if *data.get(final_index).unwrap() != ETX { //Check if last byte is ETX
                return Err(OctopipesError::BadPacket)
            }
            let mut payload = Vec::with_capacity(data_size as usize);
            for byte in &data[curr_index..final_index] {
                payload.push(*byte);
            }
            //Instance OctopipesMessage
            Ok(OctopipesMessage::new(&version, &origin, &remote, ttl, options, checksum, data))
        },
        _ => return Err(OctopipesError::UnsupportedVersion)
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
            for i in 7..1 {
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
        },
        _ => return 0
    }
    checksum
}
