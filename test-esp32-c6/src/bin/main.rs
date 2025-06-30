#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]

use core::fmt;

use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Timer};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::timer::timg::TimerGroup;
use esp_hal::{clock::CpuClock, gpio::Flex};
use esp_hal::gpio::Level;
use esp_println::println;
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
        };
    }

    fn bits_to_int(buffer: &[i32], start: usize, end: usize) -> i32 {
        let mut num = 0;

        (start..end).for_each(|i| {
            num += buffer[i]  << (end - (i + 1));
        });
        return num;
    }

    pub fn is_valid(&self) -> bool {
        return (self.humidity_decimal + self.humidity_integer + self.temperature_decimal + self.temperature_integer) & 0xFF == self.checksum
    }
}

impl fmt::Display for Dht11Reading {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return write!(f, "Humdity: {}.{}% | Temp: {}.{}C | Checksum: {}", 
            self.humidity_integer, self.humidity_decimal, self.temperature_integer, self.temperature_decimal, self.checksum
        );
    }
}


// Waits a certain amount of time with a busy loop, then returns with success and the time taken, or failure and the timeout.
// State to measure 
fn wait_with_timeout(dht_flex_pin: &Flex, timeout_micros: u64, goal_state: Level) -> (bool, Duration) {
    let start = Instant::now();
    while dht_flex_pin.level() != goal_state {
        if start.elapsed().as_micros() > timeout_micros {
            println!("We done timed out :(");
            return (false, start.elapsed())
        }

    }
    return (true, start.elapsed())
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
    let mut pulse_duration_micros = [0; 40];
    loop {
        dht_buffer.fill(0);
        pulse_duration_micros.fill(0);
        Timer::after(Duration::from_secs(5)).await;
        dht_flex_pin.set_output_enable(true);
        dht_flex_pin.set_high();
        Timer::after(Duration::from_millis(250)).await;
        println!("Starting priming sequence for DHT11 probe");
        // Start priming the dht 11 sensor to send data.
        // We must wait at least 18 ms before we can read the data. Waiting 20ms here.
        dht_flex_pin.set_low();
        println!("Set pin low");
        Timer::after(Duration::from_millis(20)).await;

        // After the minimum time has passed, we need to set the pin to high.
        dht_flex_pin.set_high();
        println!("Set pin high");
        dht_flex_pin.set_output_enable(false);
        dht_flex_pin.set_input_enable(true);

        // Wait until the DHT 11 sensors sets the pin to low
        println!("Waiting until DHT sets pin to low");
        wait_with_timeout(&dht_flex_pin, 1000, Level::Low);
        println!("DHT has set pin to low");

        // This is the prefix to basically swap ownership of the data pin from sender to reader
        println!("Waiting until DHT sets pin to high");
        wait_with_timeout(&dht_flex_pin, 1000, Level::High);
        println!("DHT has set pin to high");
        println!("Waiting until DHT sets pin to low to signal data transmission");
        println!("Now we can read bits");
        // Now we can start reading our 40 bits
        
        (0..40).for_each(|i| {
            wait_with_timeout(&dht_flex_pin, 1000, Level::High);
            let (_timed_out, time_high) = wait_with_timeout(&dht_flex_pin, 1000, Level::Low);
            pulse_duration_micros[i] = time_high.as_micros();
            dht_buffer[i] = i32::from(time_high >= Duration::from_micros(40));
        });
        
        pulse_duration_micros.iter().enumerate().for_each(|(ind, val)|
            println!("high_duration for index '{}' was '{}'", ind, val)
        );
        let dht_read = Dht11Reading::from_buffer(&dht_buffer);
        dht_flex_pin.set_input_enable(false);
        println!("Validity: {}, level read as: '{}'", dht_read.is_valid(), dht_read);
    }
    
}