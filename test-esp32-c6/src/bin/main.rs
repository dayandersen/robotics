//! Embassy DHCP Example
//!
//!
//! Set SSID and PASSWORD env variable before running this example.
//!
//! This gets an ip address via DHCP then performs an HTTP get request to some "random" server
//!
//! Because of the huge task-arena size configured this won't work on ESP32-S2

//% FEATURES: embassy esp-wifi esp-wifi/wifi esp-hal/unstable
//% CHIPS: esp32 esp32s2 esp32s3 esp32c2 esp32c3 esp32c6

#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use core::net::Ipv4Addr;

use embassy_executor::Spawner;
use embassy_net::{tcp::TcpSocket, Runner, Stack, StackResources, IpListenEndpoint};
use embassy_time::{Duration, Timer};
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{clock::CpuClock, gpio::{Level, Output, OutputConfig}, rng::Rng, timer::timg::TimerGroup};
use esp_println::println;
use esp_wifi::{
    EspWifiController,
    init,
    wifi::{ClientConfiguration, Configuration, WifiController, WifiDevice, WifiEvent, WifiState},
};
use static_cell::make_static;

mod led_control;
mod command_handler;
mod tcp_server;
mod wifi_manager;

use led_control::LedControl;
// command_handler is used by tcp_server, no direct use here needed yet unless for other commands
// use command_handler::process_command;
use tcp_server::recv_message; // Task to handle incoming TCP connections
use wifi_manager::{init_stack, connection as wifi_connection_task, SSID, PASSWORD}; // Import necessary items


esp_bootloader_esp_idf::esp_app_desc!();

// When you are okay with using a nightly compiler it's better to use https://docs.rs/static_cell/2.1.0/static_cell/macro.make_static.html
macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

// LED_ON is now managed within led_control module
// const SSID and PASSWORD are now in wifi_manager

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    // Peripherals are initialized here. `init_stack` will need some of them.
    // `LedControl::new` will also need a GPIO pin from peripherals.
    // Careful management of peripheral ownership is required.
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 96 * 1024); // Ensure this is adequate

    // Timers and RNG for Wi-Fi and Embassy
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let timg1 = TimerGroup::new(peripherals.TIMG1); // For embassy_time
    esp_hal_embassy::init(timg1.timer0); // Initialize embassy_time

    let rng = Rng::new(peripherals.RNG);
    let radio_clk = peripherals.RADIO_CLK;
    // The WIFI peripheral is moved to init_stack
    // let wifi_peripheral = peripherals.WIFI; // This line would move WIFI

    // Initialize Wi-Fi stack and spawn Wi-Fi tasks
    let stack = match wifi_manager::init_stack(spawner, peripherals, timg0, rng, radio_clk).await {
        Ok((stack_instance, _wifi_controller)) => { // wifi_controller might be needed if direct control is required later
            println!("Wi-Fi Stack initialized, SSID: {}, Password: {}", SSID, PASSWORD);
            stack_instance
        }
        Err(e) => {
            println!("Error initializing Wi-Fi stack: {:?}", e);
            panic!("Failed to initialize Wi-Fi stack"); // Or handle more gracefully
        }
    };
    
    // Initialize LED control
    // GPIO18 is needed here. Peripherals was passed to init_stack.
    // This demonstrates a common ownership challenge.
    // A proper solution would involve passing specific peripheral parts or using `unsafe { Peripherals::steal() }`
    // with caution, or restructuring how peripherals are shared/passed.
    // For this refactoring, we'll assume init_stack did not consume all of `peripherals`
    // or we re-obtain the necessary pin.  Let's use `steal` for now, highlighting it's a workaround.
    let temp_peripherals_for_led = unsafe { esp_hal::Peripherals::steal() };
    let led_pin_output = Output::new(temp_peripherals_for_led.GPIO18, Level::Low, OutputConfig::default());
    let mut led_control = LedControl::new(led_pin_output);

    // Spawn TCP server task (recv_message is now in tcp_server module)
    // It requires the stack instance.
    spawner.spawn(recv_message(stack)).expect("Failed to spawn recv_message task");

    println!("Waiting for Wi-Fi to connect and get IP address...");
    loop {
        if stack.is_link_up() {
            if let Some(config_v4) = stack.config_v4() {
                println!("Got IP: {}", config_v4.address);
                break;
            }
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    // Main loop
    println!("Entering main loop...");
    loop {
        // Update LED hardware state based on logical state from led_control
        led_control.update_hw();

        // The send_message function was removed as part of this refactoring.
        // If client functionality is needed, it should be in its own module/task.
        // For example, a new task could be spawned to periodically send messages.

        Timer::after(Duration::from_millis(1_000)).await;
    }
}

// Removed functions:
// - recv_message (moved to tcp_server.rs)
// - handle_client (moved to tcp_server.rs)
// - process_command (moved to command_handler.rs)
// - send_message (removed, was example HTTP client code)
// - connection (moved to wifi_manager.rs as a task)
// - net_task (moved to wifi_manager.rs as a task)