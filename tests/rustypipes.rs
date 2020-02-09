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
    use std::sync::{Arc, Mutex};
    use std::thread::{sleep, spawn, JoinHandle};
    use std::time::{Duration, Instant};
    #[test]
    fn server_sim() {
        //Simulates an entire server with a client
        let cap_pipe: String = String::from("/tmp/cap.fifo");
        let cap_pipe_r: String = cap_pipe.clone();
        let client_folder: String = String::from("/tmp/clients/");
        //Instance Server
        let mut server: rustypipes::OctopipesServer = rustypipes::OctopipesServer::new(
            rustypipes::OctopipesProtocolVersion::Version1,
            cap_pipe,
            client_folder,
        );
        //Set server callbacks
        server.set_on_subscription(on_subscribption);
        server.set_on_unsubscription(on_unsubscription);
        //@! Start server
        if let Err(error) = server.start_cap_listener() {
            panic!("Could not start CAP listener: {}", error);
        }
        println!("CAP Listener started...");
        let client_terminate: bool = false;
        let client_terminate_arc: Arc<Mutex<bool>> = Arc::new(Mutex::new(client_terminate));
        let client_terminate_arc2: Arc<Mutex<bool>> = Arc::clone(&client_terminate_arc);
        //Start client
        let client_join_hnd: JoinHandle<()> = spawn(move || {
            println!("Client_r thread started");
            //This client subscribes to BROADCAST and then wait for an incoming message from client_w
            let t_start: Instant = Instant::now();
            let cap_pipe_w: String = cap_pipe_r.clone();
            let mut client_r: rustypipes::OctopipesClient = rustypipes::OctopipesClient::new(
                String::from("test_client_r"),
                cap_pipe_r,
                rustypipes::OctopipesProtocolVersion::Version1,
            );
            let groups: Vec<String> = vec![String::from("TestClient"), String::from("BROADCAST")];
            loop {
                match client_r.subscribe(&groups) {
                    Ok(cap_error) => match cap_error {
                        rustypipes::OctopipesCapError::NoError => {
                            println!("Client_r subscribed (no CAP error)");
                            break;
                        }
                        _ => {
                            println!("Client_r couldn't subscribe, CAP error: {}\n", cap_error);
                            sleep(Duration::from_millis(500));
                            continue;
                        }
                    },
                    Err(error) => {
                        println!(
                            "Error while client_r was trying to subscribe to server: {}\n",
                            error
                        );
                        sleep(Duration::from_millis(500));
                        continue;
                    }
                }
            }
            println!(
                "It took {}ms for client_r to subscribe",
                t_start.elapsed().as_millis()
            );
            //Start client loop
            if let Err(error) = client_r.loop_start() {
                panic!("Couldn't start client_r loop: {}\n", error);
            }
            //@! Start WRITE client
            let client_w_join_hnd: JoinHandle<()> = spawn(move || {
                println!("Client_w thread started");
                //This client subscribes and then sends a message to test_client_r
                let t_start: Instant = Instant::now();
                let mut client_w: rustypipes::OctopipesClient = rustypipes::OctopipesClient::new(
                    String::from("test_client_w"),
                    cap_pipe_w,
                    rustypipes::OctopipesProtocolVersion::Version1,
                );
                loop {
                    match client_w.subscribe(&vec![]) {
                        Ok(cap_error) => match cap_error {
                            rustypipes::OctopipesCapError::NoError => {
                                println!("Client_w subscribed (no CAP error)");
                                break;
                            }
                            _ => {
                                println!("Client_w couldn't subscribe, CAP error: {}\n", cap_error);
                                sleep(Duration::from_millis(500));
                                continue;
                            }
                        },
                        Err(error) => {
                            println!(
                                "Error while client_w was trying to subscribe to server: {}\n",
                                error
                            );
                            sleep(Duration::from_millis(500));
                            continue;
                        }
                    }
                }
                println!(
                    "It took {}ms for client_w to subscribe",
                    t_start.elapsed().as_millis()
                );
                //Send a message
                let t_subscribed: Instant = Instant::now();
                if let Err(error) = client_w.send(
                    &String::from("TestClient"),
                    vec!['H' as u8, 'E' as u8, 'L' as u8, 'L' as u8, 'O' as u8],
                ) {
                    panic!(
                        "Error while trying to send 'HELLO' to test_client_r: {}\n",
                        error
                    );
                }
                println!(
                    "It took {}ms for client_w to send a message",
                    t_subscribed.elapsed().as_millis()
                );
                //Unsubscribe client_w
                let t_sent: Instant = Instant::now();
                if let Err(error) = client_w.unsubscribe() {
                    panic!("Error while client_w was trying to unsubscribe: {}", error);
                }
                println!(
                    "It took {}ms for client_w to unsubscribe",
                    t_sent.elapsed().as_millis()
                );
                println!(
                    "Client_w terminated after {}ms",
                    t_start.elapsed().as_millis()
                );
                //@! End of test_client_w thread
            });
            //Wait for client message
            let mut message_received: bool = false;
            println!("Client_r is now waiting for incoming messages...");
            while !message_received {
                match client_r.get_all_message() {
                    Err(error) => panic!(
                        "Error while trying to get messages on client_r: {}\n",
                        error
                    ),
                    Ok(messages) => {
                        for message in messages.iter() {
                            println!(
                                "Received message from {}: {:?}",
                                message.get_origin().unwrap(),
                                message.get_data()
                            );
                            assert_eq!(
                                *message.get_origin().unwrap(),
                                String::from("test_client_w"),
                                "Received message origin should be 'test_client_w', but is {}",
                                message.get_origin().unwrap()
                            );
                        }
                        if messages.len() > 0 {
                            message_received = true;
                        }
                    }
                }
            }
            //Unsubscribe
            let t_recv: Instant = Instant::now();
            if let Err(error) = client_r.unsubscribe() {
                panic!(
                    "Error while client_r was trying to unsubscribe: {}\n",
                    error
                );
            }
            println!(
                "It took {}ms for client_w to unsubscribe",
                t_recv.elapsed().as_millis()
            );
            //Join client w before exiting
            if let Err(err) = client_w_join_hnd.join() {
                panic!("Client W thread panic: {:?}", err);
            }
            println!(
                "Client_r terminated after {}ms",
                t_start.elapsed().as_millis()
            );
            let mut terminated = client_terminate_arc2.lock().unwrap();
            *terminated = true;
            //@! End of test_client_r thread
        });
        sleep(Duration::from_millis(100));
        //Set a timer (10s)
        let t_start_loop: Instant = Instant::now();
        //Wait for both clients subscription
        while (server
            .is_subscribed(String::from("test_client_r"))
            .is_none()
            || server
                .is_subscribed(String::from("test_client_w"))
                .is_none())
            && t_start_loop.elapsed().as_millis() < 10000
        {
            //Process cap
            if let Err(error) = server.process_cap_all() {
                panic!("Error while processing CAP: {}\n", error);
            }
            sleep(Duration::from_millis(100)); //Sleep for 100 ms
        }
        if t_start_loop.elapsed().as_millis() >= 10000 {
            panic!("Client couldn't subscribe (TIMEOUT)\n");
        }
        println!("OK, 'test_client_r and test_client_w' is subscribed");
        //Verify if client is subscribed
        let clients: Vec<String> = server.get_clients();
        assert_eq!(
            clients.len(),
            2,
            "Clients are not subscribed (clients subscribed: {})",
            clients.len()
        );
        //Verify test_client_r subscriptions
        match server.get_subscriptions(String::from("test_client_r")) {
            Some(groups) => {
                //Subscriptions should be 'TestClient' 'BROADCAST' and 'test_client_r' (cause implicit)
                assert_eq!(
                    groups[0],
                    String::from("TestClient"),
                    "Groups[0] should be 'TestClient' but is {}",
                    groups[0]
                );
                assert_eq!(
                    groups[1],
                    String::from("BROADCAST"),
                    "Groups[1] should be 'BROADCAST' but is {}",
                    groups[1]
                );
                assert_eq!(
                    groups[2],
                    String::from("test_client_r"),
                    "Groups[1] should be 'test_client_r' but is {}",
                    groups[2]
                );
            }
            None => panic!("'test_client_r' is subscribed to nothing\n"),
        }
        //Verify test_client_w subscription
        match server.get_subscriptions(String::from("test_client_w")) {
            Some(groups) => {
                //Subscriptions should be 'test_client_w' (cause implicit)
                assert_eq!(
                    groups[0],
                    String::from("test_client_w"),
                    "Groups[0] should be 'test_client_w' but is {}",
                    groups[0]
                );
            }
            None => panic!("'test_client_w' is subscribed to nothing\n"),
        }
        //@! Listen for client
        let t_start_loop = Instant::now();
        //Timeout 10 seconds
        while t_start_loop.elapsed().as_millis() < 10000 {
            //Listen for incoming CAP messages
            if let Err(error) = server.process_cap_all() {
                println!("Error while processing CAP: {}", error);
            }
            //Process workers
            if let Err((worker, error)) = server.process_all() {
                println!("Error while trying to process client {}: {}", worker, error);
            }
            let terminated = client_terminate_arc.lock().unwrap();
            if *terminated {
                break; //Break if client terminated
            }
            sleep(Duration::from_millis(100)); //Sleep for 100ms
        }
        //Force client interruption
        let mut terminated = client_terminate_arc.lock().unwrap();
        *terminated = true;
        println!(
            "It took {}ms to process all the two clients!",
            t_start_loop.elapsed().as_millis()
        );
        //Terminate client
        if let Err(err) = client_join_hnd.join() {
            panic!("Client R thread panic: {:?}\n", err);
        }
        //Stop server
        if let Err(error) = server.stop_server() {
            panic!("Could not stop Server: {}\n", error);
        }
    }

    //Callbacks
    fn on_subscribption(client_id: String) {
        println!(
            "ON_SUBSCRIBPTION_CALLBACK - Client {} subscribed!",
            client_id
        );
    }

    fn on_unsubscription(client_id: String) {
        println!(
            "ON_UNSUBSCRIBPTION_CALLBACK - Client {} unsubscribed!",
            client_id
        );
    }
}
