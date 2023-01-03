
# Rust ESP32 OSC to LED Example

Demo project to drive WS2812 (SK6812) smart led on [M5Stamp C3 Mate](https://shop.m5stack.com/collections/m5-controllers/products/m5stamp-c3-mate-with-pin-headers) via OSC with `std` feature using [`esp-idf-sys`](https://crates.io/crates/esp-idf-sys) crate

## Crates

- [rosc](https://crates.io/crates/rosc) is used to encode/decode OSC packet
- [smart-leds](https://crates.io/crates/smart-leds) trait and its implementation [ws2812-esp32-rmt-driver](https://crates.io/crates/ws2812-esp32-rmt-driver) are used to control LEDs

## Run Demo Project

First, export following environment variables in your terminal

```bash
export OSC_WIFI_SSID=your_wifi_ssid
export OSC_WIFI_PASS=your_wifi_pass
export OSC_WIFI_RECV_PORT=your_device_port_to_recv_osc
export OSC_WIFI_PONG_PORT=your_host_port_to_recv_pong
```

Next, build & flash & monitor this project to your ESP32-C3-DevKitM-1

```bash
source ~/export-esp.sh  # if needed
cargo run               # build & flash & monitor
```

With another terminal, run [`oscd`](https://crates.io/crates/oscd) to send OSC packet (IP should be your device's IP and PORT should be `OSC_WIFI_RECV_PORT`)

```bash
cargo install oscd
oscd
```

Following OSC commands are available

```bash
/ping       # reply /pong 1 to your_ip:OSC_WIFI_PONG_PORT from your device
/rgb r g b  # set color to LED (int 0-255 are available for r, g, b)
```

To monitor the `/pong` reply, open one more terminal and run `oscd` with monitor mode (listening PORT should be `OSC_WIFI_PONG_PORT`)

## Simple Examples

### `led` example

```bash
cargo run --example led
```

### `osc_ping_pong` example

Only `/ping` is available for this example

```bash
cargo run --example osc_ping_pong
```

## License

MIT
