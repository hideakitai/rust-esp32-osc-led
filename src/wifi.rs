use anyhow::{bail, Result};
use embedded_svc::wifi::*;
use esp_idf_hal::peripheral;
use esp_idf_svc::eventloop::*;
use esp_idf_svc::netif::*;
use esp_idf_svc::wifi::*;
use log::*;
use std::time::Duration;

const OSC_WIFI_TIMEOUT: Duration = Duration::from_secs(20);

pub fn init(
    modem: impl peripheral::Peripheral<P = esp_idf_hal::modem::Modem> + 'static,
    sysloop: EspSystemEventLoop,
    ssid: &str,
    pass: &str,
) -> Result<(Box<EspWifi<'static>>, embedded_svc::ipv4::Ipv4Addr)> {
    info!("Create wifi");
    let mut wifi = Box::new(EspWifi::new(modem, sysloop.clone(), None)?);

    info!("Wifi scan start");
    let ap_infos = wifi.scan()?;
    let ours = ap_infos.into_iter().find(|a| a.ssid == ssid);
    let channel = if let Some(ours) = ours {
        info!("Found AP {ssid} on channel {}", ours.channel);
        Some(ours.channel)
    } else {
        info!("AP {ssid} not found, go with unknown channel");
        None
    };

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: ssid.into(),
        password: pass.into(),
        channel,
        ..Default::default()
    }))?;

    wifi.start()?;
    info!("Starting wifi...");

    if !WifiWait::new(&sysloop)?.wait_with_timeout(OSC_WIFI_TIMEOUT, || wifi.is_started().unwrap())
    {
        bail!("Wifi did not start");
    }

    loop {
        info!("Connecting wifi...");
        wifi.connect()?;

        if EspNetifWait::new::<EspNetif>(wifi.sta_netif(), &sysloop)?.wait_with_timeout(
            OSC_WIFI_TIMEOUT,
            || {
                let is_wifi_connected = wifi.is_connected().unwrap();
                let ip = wifi.sta_netif().get_ip_info().unwrap().ip;
                is_wifi_connected && ip != std::net::Ipv4Addr::new(0, 0, 0, 0)
            },
        ) {
            info!("Wifi connection success");
            break;
        } else {
            warn!("Wifi did not connect or did not receive a DHCP lease: retry...");
        }
    }

    let ip_info = wifi.sta_netif().get_ip_info()?;
    info!("Wifi DHCP info: {ip_info:?}");

    Ok((wifi, ip_info.ip))
}

pub fn deinit(wifi: Box<EspWifi<'static>>) {
    drop(wifi);
    info!("Wifi stopped");
}
