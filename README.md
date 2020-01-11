# Rustypipes

Current Version: 0.1.0 (11/01/2020)
Developed by *Christian Visintin*

Rustypipes is a Rust library to implement Octopipes clients and servers.

```sh
cargo add rustypipes 0.1.0
```

or

```toml
[dependencies]
rustypipes = "0.1.0"
```

- [Rustypipes](#rustypipes)
  - [Client Implementation](#client-implementation)
  - [Server Implementation](#server-implementation)
  - [License](#license)

The library documentation can be found at <https://docs.rs/unix-named-pipe/0.1.0/rustypipes/>

## Client Implementation

Implementing an Octopipes Client is pretty much simple and requires only the following steps:

Initialize the client

```rust
let cap_pipe: String = String::from("/tmp/cap.fifo");
let mut client: rustypipes::OctopipesClient = rustypipes::OctopipesClient::new(
    String::from("myclient-123somerandomchars456"),
    cap_pipe,
    rustypipes::OctopipesProtocolVersion::Version1,
);
```

Subscribe to the server

```rust
let my_groups: Vec<String> = vec![String::from("myclient"), String::from("BROADCAST")];
match client.subscribe(&my_groups) {
  Ok(cap_error) => {
      match cap_error {
          rustypipes::OctopipesCapError::NoError => {
              println!("client subscribed (no CAP error)");
          },
          _ => panic!("client couldn't subscribe, CAP error: {}\n", cap_error)
      }
  },
  Err(error) => panic!("Error while client was trying to subscribe to server: {}\n", error)
}
```

Wait for incoming messages

```rust
//Start client loop
if let Err(error) = client.loop_start() {
    panic!("Couldn't start client loop: {}\n", error);
}
//...
//In your main loop
loop {
    //...
    //Get first available message
    match client.get_next_message() {
        Ok(recv) => {
            match recv {
                Some(message) => {
                    println!("Received message from {}", message.origin.as_ref().unwrap());
                    //Handle message...
                },
                None => println!("No message in client's inbox")
            }
        },
        Err(error) => {
            panic!("Error while trying to get messages on client: {}\n", error)
        }
    }
    //...
}
```

Send a message

```rust
let mydata: Vec<u8> = vec![0x01, 0x02, 0x03, 0x04];
if let Err(error) = client.send(&String::from("SomeRemote"), mydata) {
    panic!("Error while trying to send data: {}\n", error);
}
```

Then, when you're done, unsubscribe from server and terminate the client

```rust
if let Err(error) = client.unsubscribe() {
    panic!("Error while client was trying to unsubscribe: {}\n", error);
}
```

## Server Implementation

Rustypipes is designed to achieve the implementation of an Octopipes Server in the easiest and simplest way possible, saving up time and line of codes to the final developer.

Initialize the Server

```rust
let cap_pipe: String = String::from("/tmp/cap.fifo");
let client_folder: String = String::from("/tmp/clients/");
let mut server: rustypipes::OctopipesServer = rustypipes::OctopipesServer::new(
    rustypipes::OctopipesProtocolVersion::Version1,
    cap_pipe,
    client_folder,
);
```

Start the CAP listener

the CAP listener will just start a thread which will listen on the CAP for new messages, once a new message is received this will be able to be retrieved with cap process functions as we'll see later.

```rust
if let Err(error) = server.start_cap_listener() {
    panic!("Could not start CAP listener: {}", error);
}
```

Server Main Loop

These two simple functions will handle for you all the tasks the server has to achieve. Try to call this functions often (about each 100ms or less).

```rust
loop {
    //This function will check if there's any message available on the CAP
    //If there's a message available it will handle the request for you automatically
    //If a new client subscribed, a new worker will start
    //If a client unsubscribed, its worker will be stopped
    //NOTE: process_cap_all can be used too, but cap_once should be preferred
    if let Err(error) = server.process_cap_once() {
        println!("Error while processing CAP: {}", error);
    }
    //This function will check for each worker if there is any message on their inbox
    //If a message is found, it'll be dispatched to the clients subscribed to that remote
    //NOTE: process_first and process_all can be used too, but process_once should be preferred
    if let Err((fault_client, error)) = server.process_once() {
        println!("Error while processing client '{}': {}", fault_client, error);
    }
}
```

Terminate the server

```rust
if let Err(error) = server.stop_server() {
    panic!("Could not stop Server: {}\n", error);
}
//You could also just call, since drop trait is implemented and stop the server gracefully
//drop(server);
```

## License

```txt
MIT License

Copyright (c) 2019-2020 Christian Visintin

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
