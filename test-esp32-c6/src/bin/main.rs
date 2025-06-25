#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]

use core::fmt;

use embedded_io_async::{Write};
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_net::{new, tcp::TcpSocket, IpListenEndpoint, Runner, Stack, StackResources};
use embassy_time::{Duration, Instant, Timer};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{clock::CpuClock, gpio::{Flex, Input, InputConfig, Level, Output, OutputConfig}, peripherals::GPIO, rng::Rng, timer::timg::{etm::Tasks, TimerGroup}};
use esp_println::println;
use esp_wifi::{
    init, wifi::{ClientConfiguration, Configuration, WifiController, WifiDevice, WifiEvent, WifiState}, EspWifiController, EspWifiTimerSource
};
use static_cell::make_static;

esp_bootloader_esp_idf::esp_app_desc!();

struct Dht11Reading {
    humidity_integer: i32,
    humidity_decimal: i32,
    temperature_integer: i32,
    temperature_decimal: i32,
    checksum: i32,
}

impl Dht11Reading {
    pub fn from_buffer(buffer: &[i32]) -> Dht11Reading {
        return Dht11Reading {
            humidity_integer: Self::bits_to_int(buffer, 0, 8),
            humidity_decimal:Self::bits_to_int(buffer, 8, 16),
            temperature_integer: Self::bits_to_int(buffer, 16, 24),
            temperature_decimal:Self::bits_to_int(buffer, 24, 32),
            checksum:Self::bits_to_int(buffer, 32, 40),
        }
        ;
    }

    fn bits_to_int(buffer: &[i32], start: usize, end: usize) -> i32 {
        let mut num = 0;

        for i in start..end {
            num += buffer[i]  << (end - (i + 1))
        }
        return num;
    }
}

impl fmt::Display for Dht11Reading {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return write!(f, "Humdity: {}.{} | Temp: {}.{} | Checksum: {}", 
            self.humidity_integer, self.humidity_decimal, self.temperature_integer, self.temperature_decimal, self.checksum
        );
    }
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
esp_println::logger::init_logger_from_env();
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_hal_embassy::init(timg0.timer0);

    esp_alloc::heap_allocator!(size: 96 * 1024);
    let mut dht_flex_pin = Flex::new(peripherals.GPIO0);
    dht_flex_pin.set_high();
    // spawner.spawn(pull_dht_data(make_static!(dht_data_pin)));
    let mut dht_buffer = [0; 40];
    loop {
        Timer::after(Duration::from_millis(2000)).await;
        println!("Starting to measure from probe");
        dht_flex_pin.set_output_enable(true);
        // Start priming the dht 11 sensor to send data.
        // We must wait at least 18 ms before we can read the data. Waiting 20ms here.
        dht_flex_pin.set_low();
        println!("Set pin low");
        Timer::after(Duration::from_millis(20)).await;

        // After the minimum time has passed, we need to set the pin to high.
        dht_flex_pin.set_high();
        println!("Set pin high");
        Timer::after(Duration::from_micros(30)).await;
        dht_flex_pin.set_output_enable(false);
        dht_flex_pin.set_input_enable(true);

        // Wait until the DHT 11 sensors sets the pin to low
        println!("Waiting until DHT sets pin to low");
        while dht_flex_pin.level() == Level::High {}
        println!("DHT has set pin to low");

        // This is the prefix to basically swap ownership of the data pin from sender to reader
        println!("Waiting until DHT sets pin to high");
        while dht_flex_pin.level() == Level::Low {}
        println!("DHT has set pin to high");
        println!("Waiting until DHT sets pin to low again");
        while dht_flex_pin.level() == Level::High {}
        println!("DHT set pin to low again");

        println!("Now we can read bits");
        // Now we can start reading our 40 bits
        for i in 0..40 {
            while dht_flex_pin.level() == Level::High {}
            while dht_flex_pin.level() == Level::Low {}
            let start = Instant::now();
            while dht_flex_pin.level() == Level::High {}

            dht_buffer[i] = i32::from(start.elapsed() >= Duration::from_micros(40));
        }

        let dht_read = Dht11Reading::from_buffer(&dht_buffer);
        dht_flex_pin.set_input_enable(false);
        println!("Level read as: '{}'", dht_read);
    }
}