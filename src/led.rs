use anyhow::Result;
use smart_leds::SmartLedsWrite;
use std::time::Duration;
use thingbuf::mpsc::StaticReceiver;
use ws2812_esp32_rmt_driver::{driver::color::LedPixelColorGrbw32, LedPixelEsp32Rmt, RGB8};

const LED_PIN: u32 = 2; // 2: M5Stamp C3 Mate, 8: ESP32-C3-DevKitM-1
const NUM_PIXELS: usize = 1;
const LED_FRAME_INTERVAL_MS: Duration = Duration::from_millis(30);

pub struct Led {
    ws2812: LedPixelEsp32Rmt<RGB8, LedPixelColorGrbw32>,
    receiver: StaticReceiver<RGB8>,
    rgb: RGB8,
}

impl Led {
    pub fn new(receiver: StaticReceiver<RGB8>) -> Self {
        let ws2812 = LedPixelEsp32Rmt::<RGB8, LedPixelColorGrbw32>::new(0, LED_PIN).unwrap();
        let rgb = RGB8 { r: 0, g: 0, b: 0 };
        Self {
            ws2812,
            receiver,
            rgb,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        // receive color via thingbuf::mpsc::StaticChannel
        if let Ok(rgb) = self.receiver.try_recv() {
            self.rgb = rgb;
        }

        // set same color to all LEDs (create iterator that returns same color)
        let pixels = std::iter::repeat(self.rgb).take(NUM_PIXELS);
        self.ws2812.write(pixels)?;

        Ok(())
    }

    pub fn idle(&self) {
        std::thread::sleep(LED_FRAME_INTERVAL_MS);
    }
}
