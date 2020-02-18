//! ## Pipes
//!
//! `pipes` is the module which takes care of interfacing with the Unix Pipes

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

#[cfg(any(unix, macos, linux))]
extern crate unix_named_pipe;

#[cfg(windows)]
extern crate windows_named_pipe;
//Pipe prefix
#[cfg(windows)]
const WINDOWS_PIPE_PREFIX: &str = "\\\\.pipe\\";

use std::io::{Error, ErrorKind, Read, Write};
use std::time::{Duration, Instant};

//@!Unix / Linux / MacOS functions

#[cfg(any(unix, macos))]
/// ### pipe_create
///
/// `pipe_create` creates a Unix Pipe in the specified path
pub(super) fn pipe_create(path: &String) -> std::io::Result<()> {
    match unix_named_pipe::create(path, Some(0o666)) {
        Ok(..) => Ok(()),
        Err(error) => {
            match error.kind() {
                ErrorKind::AlreadyExists => Ok(()), //OK if already exists
                _ => Err(error),
            }
        }
    }
}

#[cfg(any(unix, macos))]
/// ### pipe_delete
///
/// `pipe_delete` deletes a Unix Pipe in the specified path
pub(super) fn pipe_delete(path: &String) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Ok(..) => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(any(unix, macos))]
/// ### pipe_read
///
/// `pipe_read` read from pipe; Returns or if after millis nothing has been read or if there's no more data available.
/// pipe does not have to contain \\.pipe\ prefix
pub(super) fn pipe_read(path: &String, timeout_millis: u128) -> std::io::Result<Option<Vec<u8>>> {
    //Try open pipe
    let res = unix_named_pipe::open_read(path);
    if res.is_err() {
        return Err(res.err().unwrap());
    }
    let mut pipe = res.unwrap();
    let t_start = Instant::now();
    let mut time_elapsed: Duration = Duration::from_millis(0);
    let mut data_out: Vec<u8> = Vec::new();
    while time_elapsed.as_millis() < timeout_millis || timeout_millis == 0 {
        let mut buffer: [u8; 2048] = [0; 2048];
        match pipe.read(&mut buffer) {
            Ok(bytes) => {
                //Sum elapsed time
                //If 0 bytes were read:
                // - If there are already bytes in the buffer, return
                // - Otherwise continue until time_elapsed < timeout
                if bytes == 0 {
                    if data_out.len() > 0 {
                        break;
                    } else {
                        time_elapsed = t_start.elapsed(); //Sum time only if no data was received (in order to prevent cuts)
                        continue;
                    }
                }
                //Add buffer to data
                data_out.extend_from_slice(&buffer[0..bytes]);
            }
            Err(error) => {
                //Check error
                match error.kind() {
                    ErrorKind::WouldBlock => {
                        time_elapsed = t_start.elapsed();
                        continue;
                    }
                    _ => return Err(error),
                }
            }
        }
    }
    if data_out.len() > 0 {
        Ok(Some(data_out))
    } else {
        Ok(None)
    }
}

#[cfg(any(unix, macos))]
/// ### pipe_write
///
/// `pipe_write` write to pipe; Returns after millis if nothing has been written or if the entire payload has been written. ErrorKind is WriteZero if there was no endpoint reading the pipe
pub(super) fn pipe_write(
    path: &String,
    timeout_millis: u128,
    data_out: Vec<u8>,
) -> std::io::Result<()> {
    //Try open pipe
    let t_start = Instant::now();
    let mut time_elapsed: Duration = Duration::from_millis(0);
    let mut pipe_wrapper: Option<std::fs::File> = None;
    while time_elapsed.as_millis() < timeout_millis || timeout_millis == 0 {
        let res = unix_named_pipe::open_write(path);
        match res {
            Ok(file) => {
                //Pipe OPEN, break and go write
                pipe_wrapper = Some(file);
                break;
            }
            Err(err) => {
                match err.kind() {
                    ErrorKind::Other => {
                        //Continue
                        time_elapsed = t_start.elapsed();
                        continue;
                    }
                    _ => return Err(err),
                }
            }
        }
    }
    if pipe_wrapper.is_none() {
        return Err(Error::from(ErrorKind::WriteZero));
    }
    let mut bytes_written: usize = 0;
    let mut pipe: std::fs::File = pipe_wrapper.unwrap();
    while time_elapsed.as_millis() < timeout_millis || timeout_millis == 0 {
        match pipe.write(data_out.as_slice()) {
            Ok(bytes) => {
                //Sum elapsed time
                time_elapsed = t_start.elapsed();
                bytes_written += bytes;
                if bytes_written == data_out.len() {
                    break;
                } else {
                    continue;
                }
            }
            Err(error) => return Err(error),
        }
    }
    if bytes_written < data_out.len() {
        return Err(Error::from(ErrorKind::WriteZero));
    }
    Ok(())
}

//@!Windows functions

#[cfg(windows)]
/// ### pipe_create
///
/// `pipe_create` Try to create a Windows Pipe
pub(super) fn pipe_create(path: &String) -> std::io::Result<()> {
    let win_path: String = to_windows_path(path);
    let path_struct = std::path::Path::new(&win_path);
    match windows_named_pipe::PipeStream::connect(path_struct) {
        Ok(_) => Ok(()),
        Err(err) => return Err(err),
    }
}

#[cfg(windows)]
/// ### pipe_delete
///
/// `pipe_delete` On windows, does absolutely nothing
pub(super) fn pipe_delete(_path: &String) -> std::io::Result<()> {
    Ok(())
}

#[cfg(windows)]
/// ### pipe_read
///
/// `pipe_read` read from pipe; Returns or if after millis nothing has been read or if there's no more data available
pub(super) fn pipe_read(path: &String, timeout_millis: u128) -> std::io::Result<Option<Vec<u8>>> {
    //Try open pipe
    let win_path: String = to_windows_path(path);
    let path_struct = std::path::Path::new(&win_path);
    let mut pipe: windows_named_pipe::PipeStream =
        match windows_named_pipe::PipeStream::connect(path_struct) {
            Ok(stream) => stream,
            Err(err) => return Err(err),
        };
    let t_start = Instant::now();
    let mut time_elapsed: Duration = Duration::from_millis(0);
    let mut data_out: Vec<u8> = Vec::new();
    while time_elapsed.as_millis() < timeout_millis || timeout_millis == 0 {
        let mut buffer: [u8; 2048] = [0; 2048];
        match pipe.read(&mut buffer) {
            Ok(bytes) => {
                //Sum elapsed time
                //If 0 bytes were read:
                // - If there are already bytes in the buffer, return
                // - Otherwise continue until time_elapsed < timeout
                if bytes == 0 {
                    if data_out.len() > 0 {
                        break;
                    } else {
                        time_elapsed = t_start.elapsed(); //Sum time only if no data was received (in order to prevent cuts)
                        continue;
                    }
                }
                //Add buffer to data
                data_out.extend_from_slice(&buffer[0..bytes]);
            }
            Err(error) => {
                //Check error
                match error.kind() {
                    ErrorKind::WouldBlock => {
                        time_elapsed = t_start.elapsed();
                        continue;
                    }
                    _ => return Err(error),
                }
            }
        }
    }
    if data_out.len() > 0 {
        Ok(Some(data_out))
    } else {
        Ok(None)
    }
}

#[cfg(windows)]
/// ### pipe_write
///
/// `pipe_write` write to pipe; Returns after millis if nothing has been written or if the entire payload has been written. ErrorKind is WriteZero if there was no endpoint reading the pipe
pub(super) fn pipe_write(
    path: &String,
    timeout_millis: u128,
    data_out: Vec<u8>,
) -> std::io::Result<()> {
    //Try open pipe
    let t_start = Instant::now();
    let mut time_elapsed: Duration = Duration::from_millis(0);
    let mut pipe_wrapper: Option<windows_named_pipe::PipeStream> = None;
    let win_path: String = to_windows_path(path);
    let path_struct = std::path::Path::new(&win_path);
    while time_elapsed.as_millis() < timeout_millis || timeout_millis == 0 {
        let res = windows_named_pipe::PipeStream::connect(path_struct);
        match res {
            Ok(stream) => {
                //Pipe OPEN, break and go write
                pipe_wrapper = Some(stream);
                break;
            }
            Err(err) => {
                match err.kind() {
                    ErrorKind::Other => {
                        //Continue
                        time_elapsed = t_start.elapsed();
                        continue;
                    }
                    _ => return Err(err),
                }
            }
        }
    }
    if pipe_wrapper.is_none() {
        return Err(Error::from(ErrorKind::WriteZero));
    }
    let mut bytes_written: usize = 0;
    let mut pipe: windows_named_pipe::PipeStream = pipe_wrapper.unwrap();
    while time_elapsed.as_millis() < timeout_millis || timeout_millis == 0 {
        match pipe.write(data_out.as_slice()) {
            Ok(bytes) => {
                //Sum elapsed time
                time_elapsed = t_start.elapsed();
                bytes_written += bytes;
                if bytes_written == data_out.len() {
                    break;
                } else {
                    continue;
                }
            }
            Err(error) => return Err(error),
        }
    }
    if bytes_written < data_out.len() {
        return Err(Error::from(ErrorKind::WriteZero));
    }
    Ok(())
}

#[cfg(windows)]
fn to_windows_path(path: &String) -> String {
    if path.starts_with(WINDOWS_PIPE_PREFIX) {
        String::from(path)
    } else {
        format!("{}{}", WINDOWS_PIPE_PREFIX, path)
    }
}

//@! Tests

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_pipe_create_and_delete() {
        //Try to create a pipe in /tmp/pipe_test
        if let Err(ioerr) = pipe_create(&String::from("/tmp/pipe_test")) {
            panic!("Could not create pipe: {}", ioerr)
        }
        //Try to recreate
        if let Err(ioerr) = pipe_create(&String::from("/tmp/pipe_test")) {
            panic!("Could not create pipe: {}", ioerr)
        }
        //Then delete it
        if let Err(ioerr) = pipe_delete(&String::from("/tmp/pipe_test")) {
            panic!("Could not delete previously created pipe: {}", ioerr)
        }
        if let Ok(_) = pipe_delete(&String::from("/tmp/pipe_test")) {
            panic!("Pipe_delete returned oK while removing already deleted pipe")
        }
    }

    #[test]
    fn test_pipe_io() {
        //Create tx and rx pipes
        let pipe_tx: String = String::from("/tmp/pipe_tx");
        let pipe_rx: String = String::from("/tmp/pipe_rx");
        match pipe_create(&pipe_tx) {
            Ok(_) => println!("Pipe created with success"),
            Err(ioerr) => panic!("Could not create pipe: {}", ioerr),
        }
        match pipe_create(&pipe_rx) {
            Ok(_) => println!("Pipe created with success"),
            Err(ioerr) => panic!("Could not create pipe: {}", ioerr),
        }
        //Create a thread for write
        let pipe_rx_copy: String = pipe_rx.clone();
        let join_hnd: thread::JoinHandle<()> = thread::spawn(move || {
            //Prepare data (255 bytes, from 0 to ff)
            let mut data: Vec<u8> = Vec::with_capacity(255);
            for i in 0..255 {
                data.push(i);
            }
            //Write data
            match pipe_write(&pipe_rx_copy, 5000, data) {
                Ok(()) => println!("Successfully wrote 255 bytes to pipe rx"),
                Err(ioerr) => panic!("Could not write to pipe: {}", ioerr),
            }
        });
        //Read
        match pipe_read(&pipe_rx, 5000) {
            Ok(data_opt) => {
                if data_opt.is_none() {
                    panic!("Data shouldn't be None");
                }
                let data: Vec<u8> = data_opt.unwrap();
                //Print data
                print!("Pipe Read - Successfully read data: ");
                for byte in &data {
                    print!("{:02x} ", byte);
                }
                println!("\n");
                assert_eq!(
                    data.len(),
                    255,
                    "Pipe read: data len should be 255, but is {}",
                    data.len()
                );
            }
            Err(ioerr) => panic!("Error while reading from pipe: {}", ioerr),
        }
        join_hnd.join().expect("Could not join write thread");
        println!("Write thread ended");
        //Delete pipes
        match pipe_delete(&pipe_tx) {
            Ok(_) => println!("Pipe deleted with success"),
            Err(ioerr) => panic!("Could not delete previously created pipe: {}", ioerr),
        }
        match pipe_delete(&pipe_rx) {
            Ok(_) => println!("Pipe deleted with success"),
            Err(ioerr) => panic!("Could not delete previously created pipe: {}", ioerr),
        }
    }

    #[test]
    fn test_pipe_read_no_endpoint() {
        //Try to create a pipe in /tmp/pipe_test
        match pipe_create(&String::from("/tmp/pipe_read_noendpoint")) {
            Ok(_) => println!("Pipe created with success"),
            Err(ioerr) => panic!("Could not create pipe: {}", ioerr),
        }
        //Read (Should return after 3 seconds)
        let t_start = Instant::now();
        match pipe_read(&String::from("/tmp/pipe_read_noendpoint"), 3000) {
            Ok(data_opt) => {
                if data_opt.is_none() {
                    println!("Ok, data is None as expected");
                } else {
                    panic!("Data shouldn't be some!");
                }
                let elapsed_time: Duration = t_start.elapsed();
                assert!(
                    elapsed_time.as_millis() >= 3000,
                    "Elapsed time should be at least 3000ms, but is {}",
                    elapsed_time.as_millis()
                );
            }
            Err(ioerr) => panic!("Error while reading from pipe with NO ENDPOINT: {}", ioerr),
        }
        //Then delete it
        match pipe_delete(&String::from("/tmp/pipe_read_noendpoint")) {
            Ok(_) => println!("Pipe deleted with success"),
            Err(ioerr) => panic!("Could not delete previously created pipe: {}", ioerr),
        }
    }

    #[test]
    fn test_pipe_write_no_endpoint() {
        //Try to create a pipe in /tmp/pipe_test
        match pipe_create(&String::from("/tmp/pipe_write_noendpoint")) {
            Ok(_) => println!("Pipe created with success"),
            Err(ioerr) => panic!("Could not create pipe: {}", ioerr),
        }
        //Write (Should return after 3 seconds)
        let t_start = Instant::now();
        match pipe_write(
            &String::from("/tmp/pipe_write_noendpoint"),
            3000,
            vec![0x00, 0x01, 0x02, 0x03],
        ) {
            Ok(_) => {
                panic!("Pipe write without end point should have returned error (WriteZero), but returned OK");
            }
            Err(ioerr) => match ioerr.kind() {
                ErrorKind::WriteZero => {
                    let elapsed_time: Duration = t_start.elapsed();
                    assert!(
                        elapsed_time.as_millis() >= 3000,
                        "Elapsed time should be at least 3000ms, but is {}",
                        elapsed_time.as_millis()
                    );
                }
                _ => panic!(
                    "Error kind should be WriteZero, but is {} ({:?})",
                    ioerr,
                    ioerr.kind()
                ),
            },
        }
        //Then delete it
        match pipe_delete(&String::from("/tmp/pipe_write_noendpoint")) {
            Ok(_) => println!("Pipe deleted with success"),
            Err(ioerr) => panic!("Could not delete previously created pipe: {}", ioerr),
        }
    }

    #[test]
    fn test_pipe_read_bad() {
        if let Ok(_) = pipe_read(&String::from("/tmp/not-existing-pipe"), 500) {
            panic!("Read from not-existing pipe returned oK");
        }
    }
}
