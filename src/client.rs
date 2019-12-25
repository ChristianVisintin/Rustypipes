//! ## Client
//!
//! `client` is the module which takes care of managin the OctopipesClient struct and
//! then all the functions useful for the user to interface with an Octopipes Server

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

use super::OctopipesClient;
use super::OctopipesError;
use super::OctopipesMessage;
use super::OctopipesOptions;
use super::OctopipesProtocolVersion;
use super::OctopipesState;

use std::sync::{Arc, Mutex};
use std::thread;

impl OctopipesClient {
  /// ### OctopipesClient Constructor
  ///
  /// `new` is constructor for OctopipesClient
  /// #### Examples
  /// ```
  /// let mut client = OctopipesClient::new("test_client", "/tmp/pipe_cap", OctopipesProtocolVersion::Version1);
  /// ```
  pub fn new(
    client_id: String,
    cap_pipe: String,
    version: OctopipesProtocolVersion,
  ) -> OctopipesClient {
    OctopipesClient {
      id: client_id,
      version: version,
      cap_pipe: cap_pipe,
      tx_pipe: None,
      rx_pipe: None,
      state: OctopipesState::Initialized,
      client_loop: None,
      on_received_fn: None,
      on_sent_fn: None,
      on_subscribed_fn: None,
      on_unsubscribed_fn: None,
    }
  }

  pub fn clone(client: &OctopipesClient) -> OctopipesClient {
    OctopipesClient {
      id: client.id.clone(),
      version: client.version.clone(),
      cap_pipe: client.cap_pipe.clone(),
      tx_pipe: client.tx_pipe.clone(),
      rx_pipe: client.rx_pipe.clone(),
      state: client.state.clone(),
      client_loop: None,
      on_received_fn: client.on_received_fn,
      on_sent_fn: client.on_sent_fn,
      on_subscribed_fn: client.on_subscribed_fn,
      on_unsubscribed_fn: client.on_unsubscribed_fn,
    }
  }

  //Thread operations

  /// #### Examples
  /// ```
  /// match my_client.loop_start() {
  ///     Ok(_state) => {
  ///         println!("Thread started!");
  ///     },
  ///     Error(err) => {
  ///         println!("Error while trying to start thread");
  ///     }
  /// }
  /// ```
  pub fn loop_start(&mut self) -> Result<OctopipesState, OctopipesError> {
    match self.state {
      OctopipesState::Subscribed => {
        //Create threaded client
        //Share state between threads
        let shared_state = Arc::new(Mutex::new(self.state));
        //Clone client for thread
        let mut threaded_client: OctopipesClient = OctopipesClient::clone(self);
        let shared_state = Arc::clone(&shared_state);
        //Start thread
        self.client_loop = Some(thread::spawn(move || {
          //TODO: implement thread
          let current_state = *shared_state.lock().unwrap();
          threaded_client.state = current_state;
          match threaded_client.state {
            OctopipesState::Running => {
              //TODO: read
            }
            _ => {
              //Exit
            }
          };
        }));
        self.state = OctopipesState::Running;
        Ok(OctopipesState::Running)
      }
      OctopipesState::Running => Err(OctopipesError::ThreadAlreadyRunning),
      _ => Err(OctopipesError::NotSubscribed),
    }
  }

  /// #### Examples
  /// ```
  /// match my_client.loop_stop() {
  ///     Ok(_state) => {
  ///         println!("Thread stopped!");
  ///     },
  ///     Error(err) => {
  ///         println!("Error while trying to stop thread");
  ///     }
  /// }
  /// ```
  pub fn loop_stop(&mut self) -> Result<OctopipesState, OctopipesError> {
    match self.state {
      OctopipesState::Running => {
        //Stop thread
        self.state = OctopipesState::Stopped;
        //Take joinable out of Option and then Join thread (NOTE: Using take prevents errors!)
        self.client_loop.take().map(thread::JoinHandle::join);
        Ok(OctopipesState::Stopped)
      }
      _ => Ok(OctopipesState::Subscribed),
    }
  }
}
