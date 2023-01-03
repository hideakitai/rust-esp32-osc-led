use anyhow::{bail, Result};
use embedded_svc::wifi::*;
use esp_idf_hal::peripheral;
use esp_idf_hal::prelude::*;
use esp_idf_svc::eventloop::*;
use esp_idf_svc::netif::*;
use esp_idf_svc::wifi::*;
use esp_idf_sys;
use log::*;
use rosc::{self, OscMessage, OscPacket, OscType};
use std::net::{SocketAddr, SocketAddrV4, UdpSocket};
use std::str::FromStr;
use std::{env, time::*};

const OSC_WIFI_SSID: &str = env!("OSC_WIFI_SSID");
const OSC_WIFI_PASS: &str = env!("OSC_WIFI_PASS");
const OSC_WIFI_RECV_PORT_STR: &str = env!("OSC_WIFI_RECV_PORT");
const OSC_WIFI_PONG_PORT_STR: &str = env!("OSC_WIFI_PONG_PORT");
const OSC_WIFI_TIMEOUT: Duration = Duration::from_secs(20);

fn main() -> Result<()> {
    // Initialize nvs
    unsafe {
        esp_idf_sys::nvs_flash_init();
    }

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    // Initialize Wi-Fi and connect to AP
    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let wifi = init_wifi(peripherals.modem, sysloop.clone())?;

    // Create socket for osc
    let ip_info = wifi.sta_netif().get_ip_info()?;
    let recv_port = OSC_WIFI_RECV_PORT_STR.parse::<u16>().unwrap();
    let recv_addr = SocketAddrV4::new(ip_info.ip, recv_port);
    let sock = UdpSocket::bind(recv_addr).unwrap();
    info!("Listening to {recv_addr}");

    // Receive osc
    let mut buf = [0u8; rosc::decoder::MTU];
    let pong_port = OSC_WIFI_PONG_PORT_STR.parse::<u16>().unwrap();
    loop {
        match sock.recv_from(&mut buf) {
            Ok((size, addr)) => {
                info!("Received packet with size {size} from: {addr}");
                let (_, packet) = rosc::decoder::decode_udp(&buf[..size]).unwrap();
                let mut pong_addr = addr.clone();
                pong_addr.set_port(pong_port);
                handle_osc_packet(packet, &sock, pong_addr);
            }
            Err(e) => {
                error!("Error receiving from socket: {e}");
                break;
            }
        }
    }

    drop(wifi);
    info!("Wifi stopped");

    Ok(())
}

fn init_wifi(
    modem: impl peripheral::Peripheral<P = esp_idf_hal::modem::Modem> + 'static,
    sysloop: EspSystemEventLoop,
) -> Result<Box<EspWifi<'static>>> {
    let mut wifi = Box::new(EspWifi::new(modem, sysloop.clone(), None)?);
    info!("Wifi created, about to scan");

    let ap_infos = wifi.scan()?;
    let ours = ap_infos.into_iter().find(|a| a.ssid == OSC_WIFI_SSID);
    let channel = if let Some(ours) = ours {
        info!(
            "Found configured AP {} on channel {}",
            OSC_WIFI_SSID, ours.channel
        );
        Some(ours.channel)
    } else {
        info!(
            "Configured AP {} not found during scanning, will go with unknown channel",
            OSC_WIFI_SSID
        );
        None
    };

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: OSC_WIFI_SSID.into(),
        password: OSC_WIFI_PASS.into(),
        channel,
        ..Default::default()
    }))?;

    wifi.start()?;
    info!("Starting wifi...");

    if !WifiWait::new(&sysloop)?.wait_with_timeout(OSC_WIFI_TIMEOUT, || wifi.is_started().unwrap())
    {
        bail!("Wifi did not start");
    }

    info!("Connecting wifi...");
    wifi.connect()?;

    if !EspNetifWait::new::<EspNetif>(wifi.sta_netif(), &sysloop)?.wait_with_timeout(
        OSC_WIFI_TIMEOUT,
        || {
            let is_wifi_connected = wifi.is_connected().unwrap();
            let ip = wifi.sta_netif().get_ip_info().unwrap().ip;
            is_wifi_connected && ip != std::net::Ipv4Addr::new(0, 0, 0, 0)
        },
    ) {
        bail!("Wifi did not connect or did not receive a DHCP lease");
    }

    let ip_info = wifi.sta_netif().get_ip_info()?;
    info!("Wifi DHCP info: {ip_info:?}");

    Ok(wifi)
}

fn handle_osc_packet(packet: OscPacket, sock: &UdpSocket, pong_addr: SocketAddr) {
    match packet {
        OscPacket::Message(msg) => {
            info!("OSC address: {}", msg.addr);
            info!("OSC arguments: {:?}", msg.args);

            // reply /pong 1 to sender (port will be changed to OSC_DEST_PORT)
            if msg.addr == "/ping" {
                info!("Reply /pong to {pong_addr}");

                let msg_buf = rosc::encoder::encode(&OscPacket::Message(OscMessage {
                    addr: "/pong".to_string(),
                    args: vec![OscType::Int(1)],
                }))
                .unwrap();

                sock.send_to(&msg_buf, pong_addr).unwrap();
            }
        }
        OscPacket::Bundle(bundle) => {
            info!("OSC Bundle: {bundle:?}");
        }
    }
}
