//! ## Integration Tests
//!
//! The integration test consists in simulating and entire server with a client

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

extern crate rustypipes;

#[cfg(test)]
mod tests {
    use std::thread::{JoinHandle, spawn};
    #[test]
        fn server_sim() {
        //Simulates an entire server with a client
        let cap_pipe: String = String::from("/tmp/cap.fifo");
        let client_folder: String = String::from("/tmp/");
        //Instance Server
        let mut server: rustypipes::OctopipesServer = rustypipes::OctopipesServer::new(rustypipes::OctopipesProtocolVersion::Version1, cap_pipe, client_folder);
        //Start server
        if let Err(error) = server.start_cap_listener() {
            panic!("Could not start CAP listener: {}", error);
        }
        //Start client
        let client_join_hnd: JoinHandle<()> = spawn(move || {

        });
        //Listen for client

        client_join_hnd.join();
        //Stop server
        if let Err(error) = server.stop_cap_listener() {
            panic!("Could not stop CAP listener: {}", error);
        }
    }
}
