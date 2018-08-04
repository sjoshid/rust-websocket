#[macro_use]
extern crate serde_derive;
extern crate websocket;
extern crate serde;
extern crate serde_json;

use websocket::ws::dataframe::DataFrame;
use std::thread;
use std::sync::mpsc::channel;
use std::io::stdin;

use websocket::{Message, OwnedMessage};
use websocket::client::ClientBuilder;
use websocket::header::{Headers, Cookie};
use std::env;

const CONNECTION: &'static str = "ws://127.0.0.1:2794";

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageDetails {
    pub sender_username: String,
    pub receiver_username: String,
    pub message: String,
}

fn main() {
    println!("Connecting to {}", CONNECTION);

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        panic!("Sender is required.");
    }

    let mut my_headers = Headers::new();

    let user_id_cookie = "user_id=".to_owned() + &args[1];
    my_headers.set(Cookie(vec![user_id_cookie]));

    let client = ClientBuilder::new(CONNECTION)
        .unwrap()
        .add_protocol("rust-websocket")
        .custom_headers(&my_headers)
        .connect_insecure()
        .unwrap();

    println!("Successfully connected");

    let (mut receiver, mut sender) = client.split().unwrap();

    let (tx, rx) = channel();

    let tx_1 = tx.clone();

    let send_loop = thread::spawn(move || {
        loop {
            // Send loop
            let message = match rx.recv() {
                Ok(m) => m,
                Err(e) => {
                    println!("Send Loop: {:?}", e);
                    return;
                }
            };
            match message {
                OwnedMessage::Close(_) => {
                    let _ = sender.send_message(&message);
                    // If it's a close message, just send it and then return.
                    return;
                }
                OwnedMessage::Text(str) => {
                    let user_id = args[1].clone();
                    let message = generate_detailed_message(str, user_id);
                    // Send the message
                    match sender.send_message(&message) {
                        Ok(()) => (),
                        Err(e) => {
                            println!("Send Loop: {:?}", e);
                            let _ = sender.send_message(&Message::close());
                            return;
                        }
                    }
                },
                _ => {
                    panic!("Only Text messages are supported.");
                }
            }
        }
    });

    let receive_loop = thread::spawn(move || {
        // Receive loop
        for message in receiver.incoming_messages() {
            let message = match message {
                Ok(m) => m,
                Err(e) => {
                    println!("Receive Loop: {:?}", e);
                    let _ = tx_1.send(OwnedMessage::Close(None));
                    return;
                }
            };
            match message {
                OwnedMessage::Close(_) => {
                    // Got a close message, so send a close message and return
                    let _ = tx_1.send(OwnedMessage::Close(None));
                    return;
                }
                OwnedMessage::Ping(data) => {
                    match tx_1.send(OwnedMessage::Pong(data)) {
                        // Send a pong in response
                        Ok(()) => (),
                        Err(e) => {
                            println!("Receive Loop: {:?}", e);
                            return;
                        }
                    }
                }
                // Say what we received
                _ => println!("Receive Loop: {:?}", message),
            }
        }
    });

    loop {
        let mut input = String::new();

        stdin().read_line(&mut input).unwrap();

        let trimmed = input.trim();

        let message = match trimmed {
            "/close" => {
                // Close the connection
                let _ = tx.send(OwnedMessage::Close(None));
                break;
            }
            // Send a ping
            "/ping" => OwnedMessage::Ping(b"PING".to_vec()),
            // Otherwise, just send text
            _ => OwnedMessage::Text(trimmed.to_string()),
        };

        match tx.send(message) {
            Ok(()) => (),
            Err(e) => {
                println!("Main Loop: {:?}", e);
                break;
            }
        }
    }

    // We're exiting

    println!("Waiting for child threads to exit");

    let _ = send_loop.join();
    let _ = receive_loop.join();

    println!("Exited");
}

fn generate_detailed_message(om: String, user_id: String) -> OwnedMessage {
    //let mfs = String::from_utf8(om.take_payload()).unwrap();
    let detailed_message = MessageDetails { sender_username: user_id, message: om, receiver_username: String::from("Morgan") };

    let serialized_detailed_message = serde_json::to_string(&detailed_message).unwrap();

    OwnedMessage::Text(serialized_detailed_message)
}
