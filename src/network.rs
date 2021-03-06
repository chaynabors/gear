// Copyright 2021 Chay Nabors.

use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::sleep;
use std::thread::JoinHandle;
use std::thread::{self,};
use std::time::Duration;
use std::time::Instant;

use crossbeam::channel::Receiver;
use crossbeam::channel::Sender;
use crossbeam::channel::TryRecvError;
pub use laminar::Config as NetworkConfig;
pub use laminar::Packet;
use laminar::SocketEvent;

use crate::Result;

#[derive(Clone, Debug)]
pub enum NetworkEvent {
    Message(Packet),
    Connect(SocketAddr),
    Timeout(SocketAddr),
    Disconnect(SocketAddr),
}

#[derive(Debug)]
pub struct Socket {
    sender: Sender<Packet>,
    stop_signal: Arc<AtomicBool>,
}

impl Socket {
    pub fn send(&self, packet: Packet) -> &Self {
        self.sender.send(packet).unwrap();
        self
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        self.stop_signal.swap(true, Ordering::Relaxed);
    }
}

#[derive(Default, Debug)]
pub struct Network {
    socket_thread: Option<JoinHandle<()>>,
    receiver: Option<Receiver<SocketEvent>>,
}

impl Network {
    pub(crate) fn new() -> Self {
        Network::default()
    }

    pub(crate) fn get_event(&mut self) -> Option<NetworkEvent> {
        if let Some(receiver) = &self.receiver {
            loop {
                match receiver.try_recv() {
                    Ok(message) => {
                        return Some(match message {
                            SocketEvent::Packet(packet) => NetworkEvent::Message(packet),
                            SocketEvent::Connect(address) => NetworkEvent::Connect(address),
                            SocketEvent::Timeout(address) => NetworkEvent::Timeout(address),
                            SocketEvent::Disconnect(address) => NetworkEvent::Disconnect(address),
                        })
                    },
                    Err(e) => match e {
                        TryRecvError::Empty => break,
                        TryRecvError::Disconnected => {
                            self.socket_thread.take().unwrap().join().unwrap();
                            self.receiver.take();
                            break;
                        },
                    },
                }
            }
        }

        None
    }

    pub fn bind<A: ToSocketAddrs>(&mut self, addresses: A) -> Result<Socket> {
        self.bind_with_config(addresses, NetworkConfig::default())
    }

    pub fn bind_with_config<A: ToSocketAddrs>(&mut self, addresses: A, config: NetworkConfig) -> Result<Socket> {
        let mut socket = laminar::Socket::bind_with_config(addresses, config)?;
        let sender = socket.get_packet_sender();
        let receiver = socket.get_event_receiver();
        let stop_signal = Arc::new(AtomicBool::new(false));
        let stop = stop_signal.clone();

        let socket_thread = thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                socket.manual_poll(Instant::now());
                sleep(Duration::from_millis(1));
            }
        });

        self.socket_thread = Some(socket_thread);
        self.receiver = Some(receiver);

        Ok(Socket { sender, stop_signal })
    }
}
