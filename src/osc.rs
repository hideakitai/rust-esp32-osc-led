use anyhow::{bail, Result};
use log::*;
use rosc::{self, OscMessage, OscPacket, OscType};
use smart_leds::RGB8;
use std::net::{SocketAddrV4, UdpSocket};
use thingbuf::mpsc::StaticSender;

pub struct Osc {
    sock: UdpSocket,
    buf: [u8; rosc::decoder::MTU],
    pong_port: u16,
    sender: StaticSender<RGB8>,
}

impl Osc {
    pub fn new(
        ip: embedded_svc::ipv4::Ipv4Addr,
        recv_port: u16,
        pong_port: u16,
        sender: StaticSender<RGB8>,
    ) -> Self {
        let recv_addr = SocketAddrV4::new(ip, recv_port);
        let sock = UdpSocket::bind(recv_addr).unwrap();
        let buf = [0u8; rosc::decoder::MTU];
        info!("Listening to {recv_addr}");

        Self {
            sock,
            buf,
            pong_port,
            sender,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        match self.sock.recv_from(&mut self.buf) {
            Ok((size, addr)) => {
                info!("Received packet with size {size} from: {addr}");
                let (_, packet) = rosc::decoder::decode_udp(&self.buf[..size]).unwrap();
                match packet {
                    OscPacket::Message(msg) => {
                        info!("OSC address: {}", msg.addr);
                        info!("OSC arguments: {:?}", msg.args);

                        match msg.addr.as_str() {
                            // reply /pong 1 to sender (port will be changed to OSC_DEST_PORT)
                            "/ping" => {
                                let msg_buf =
                                    rosc::encoder::encode(&OscPacket::Message(OscMessage {
                                        addr: "/pong".to_string(),
                                        args: vec![OscType::Int(1)],
                                    }))?;

                                let mut pong_addr = addr.clone();
                                pong_addr.set_port(self.pong_port);

                                info!("Reply /pong to {pong_addr}");
                                self.sock.send_to(&msg_buf, pong_addr)?;
                            }
                            // send color via thingbuf::mpsc::StaticChannel
                            "/rgb" if msg.args.len() == 3 => {
                                let mut rgb = vec![];
                                for arg in msg.args {
                                    rgb.push(arg.int().unwrap() as u8);
                                }
                                self.sender.try_send(RGB8::from_iter(rgb))?;
                            }
                            _ => {}
                        }
                    }
                    OscPacket::Bundle(bundle) => {
                        info!("OSC Bundle: {bundle:?}");
                    }
                }
                Ok(())
            }
            Err(e) => {
                bail!("Error receiving from socket: {e}");
            }
        }
    }
}
