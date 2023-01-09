use anyhow::Result;
use esp_idf_hal::prelude::*;
use esp_idf_svc::eventloop::*;
use esp_idf_sys;
use log::*;
use smart_leds::RGB8;
use std::env;
use heapless::spsc::Queue;

mod led;
mod osc;
mod wifi;

use led::Led;
use osc::Osc;

// Load config from environment variables
const OSC_WIFI_SSID: &str = env!("OSC_WIFI_SSID");
const OSC_WIFI_PASS: &str = env!("OSC_WIFI_PASS");
const OSC_WIFI_RECV_PORT_STR: &str = env!("OSC_WIFI_RECV_PORT");
const OSC_WIFI_PONG_PORT_STR: &str = env!("OSC_WIFI_PONG_PORT");

// QUEUE should be 'static to pass to threads
static mut QUEUE: Option<Queue<RGB8, 16>> = None;

fn main() -> Result<()> {
    // Initialize nvs
    unsafe {
        esp_idf_sys::nvs_flash_init();
    }

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    // Initialize Wi-Fi and connect to AP
    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take().unwrap();
    let (wifi, ip) = wifi::init(
        peripherals.modem,
        sysloop.clone(),
        OSC_WIFI_SSID,
        OSC_WIFI_PASS,
    )?;

    // Create Queue to send RGB data from OSC thread to LED thread
    unsafe { QUEUE = Some(Queue::new()); }
    let (producer, consumer) = unsafe { QUEUE.as_mut().unwrap().split() };

    // Create thread to handle LEDs
    let led_join_handle = std::thread::Builder::new()
        .stack_size(4096)
        .spawn(move || {
            let mut led = Led::new(consumer);
            loop {
                if let Err(e) = led.run() {
                    error!("Failed to run LEDs: {e}");
                    break;
                }
                led.idle();
            }
        })?;

    // Create thread to receive/send OSC
    // Larger stack size is required (default is 3 KB)
    // You can customize default value by CONFIG_ESP_SYSTEM_EVENT_TASK_STACK_SIZE in sdkconfig
    let recv_port = OSC_WIFI_RECV_PORT_STR.parse::<u16>().unwrap();
    let pong_port = OSC_WIFI_PONG_PORT_STR.parse::<u16>().unwrap();
    let osc_join_handle = std::thread::Builder::new()
        .stack_size(8192)
        .spawn(move || {
            let mut osc = Osc::new(ip, recv_port, pong_port, producer);
            loop {
                if let Err(e) = osc.run() {
                    error!("Failed to run OSC: {e}");
                    break;
                }
            }
        })?;

    led_join_handle.join().unwrap();
    osc_join_handle.join().unwrap();

    wifi::deinit(wifi);

    info!("Finish app");
    Ok(())
}
