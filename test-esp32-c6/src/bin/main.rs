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
#![feature(impl_trait_in_assoc_type)]

use core::net::Ipv4Addr;
use embedded_io_async::{Write};
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
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

use picoserve::{request::Path, response::{ws::Control, IntoResponse}, routing::{get, parse_path_segment, post}, AppRouter, AppWithStateBuilder};
use picoserve::extract::State;

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

static mut LED_ON: bool = false;

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");
const WEB_TASK_POOL_SIZE: usize = 8;

#[derive(Clone, Copy)]
struct SharedControl(&'static Mutex<CriticalSectionRawMutex, WifiController<'static>>);

struct AppState {
    shared_controller: SharedControl,
}

impl picoserve::extract::FromRef<AppState> for SharedControl {
    fn from_ref(state: &AppState) -> Self {
        state.shared_controller
    }
}

struct AppProps;

impl AppWithStateBuilder for AppProps {
    type State = AppState;
    type PathRouter = impl picoserve::routing::PathRouter<AppState>;

    fn build_app(self) -> picoserve::Router<Self::PathRouter, Self::State> {
        picoserve::Router::new()
        .route("/", get(|| async move { "Hello World" }))
        .route(("/set_led", parse_path_segment::<bool>()), get(|led_mode: bool| async move {
            println!("Setting led mode to: {}", if led_mode { "ON" } else { "OFF" });
            unsafe  {LED_ON = led_mode};
        }))
    }
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 96 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let mut rng = Rng::new(peripherals.RNG);

    let esp_wifi_ctrl:&EspWifiController<'static> = make_static!(init(timg0.timer0, rng.clone(), peripherals.RADIO_CLK).unwrap());
    let (controller, interfaces) = esp_wifi::wifi::new(&esp_wifi_ctrl, peripherals.WIFI).unwrap();

    let wifi_interface = interfaces.sta;

    // Initialize embassy with the remaining timer from TIMG1
    let timg1 = TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init(timg1.timer0);

    let config = embassy_net::Config::dhcpv4(Default::default());

    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    // Init network stack
    let (stack, runner) = embassy_net::new(
        wifi_interface,
        config,
        mk_static!(StackResources<WEB_TASK_POOL_SIZE>, StackResources::<WEB_TASK_POOL_SIZE>::new()),
        seed,
    );

    let mut led_pin = Output::new(peripherals.GPIO8, Level::High, OutputConfig::default());

    spawner.must_spawn(net_task(runner));
     let shared_controller = SharedControl(
        picoserve::make_static!(Mutex<CriticalSectionRawMutex, WifiController<'static>>, Mutex::new(controller)),
    );
    spawner.must_spawn(connection(shared_controller));
    
    spawner.spawn(recv_message(stack)).ok();
    
    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    println!("Waiting to get IP address...");
    loop {
        if let Some(config) = stack.config_v4() {
            println!("Got IP: {}", config.address);
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }


    let app = picoserve::make_static!(AppRouter<AppProps>, AppProps.build_app());

    let config = picoserve::make_static!(
        picoserve::Config<Duration>,
        picoserve::Config::new(picoserve::Timeouts {
            start_read_request: Some(Duration::from_secs(5)),
            persistent_start_read_request: Some(Duration::from_secs(1)),
            read_request: Some(Duration::from_secs(1)),
            write: Some(Duration::from_secs(1)),
        })
        .keep_connection_alive()
    );

    

    for id in 0..WEB_TASK_POOL_SIZE {
        spawner.must_spawn(web_task(id, stack, app, config, AppState { shared_controller }));
    }

    loop {
        Timer::after(Duration::from_millis(1_000)).await;
        send_message(stack).await;
        unsafe  {
            if LED_ON == true {
                led_pin.set_high();
            } else {
                led_pin.set_low();
            }
        }
    }
}

#[embassy_executor::task]
async fn recv_message(stack: Stack<'static>) {
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    
    loop {
        rx_buffer.fill(0);
        tx_buffer.fill(0);

        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        
        match socket.accept(IpListenEndpoint { addr: None, port: 8080 }).await {
            Ok(_) => {
                println!("Someone is talking to me!");
                if let Err(e) = handle_client(&mut socket).await {
                    println!("Client handling error: {:?}", e);
                }
                println!("Client disconnected");
            }
            Err(e) => {
                println!("Accept error: {:?}", e);
                Timer::after(Duration::from_millis(1000)).await;
            }
        }
        
        socket.close();
        Timer::after(Duration::from_millis(100)).await;
    }
}

async fn handle_client(socket: &mut TcpSocket<'_>) -> Result<(), embassy_net::tcp::Error> {
    
    // Send welcome message
    socket.write_all(b"ESP32 Command Server Ready\r\nAvailable commands: LED_ON, LED_OFF, STATUS, PING\r\n> ").await?;

    let mut buffer = [0u8; 256];
    let mut pos = 0;

    loop {
        match socket.read(&mut buffer[pos..]).await {
            Ok(0) => {
                println!("Client disconnected (EOF)");
                break;
            }
            Ok(len) => {
                pos += len;
                
                // Look for complete command (ended with \r\n or \n)
                if let Some(newline_pos) = buffer[..pos].iter().position(|&b| b == b'\n') {
                    let command_bytes = &buffer[..newline_pos];
                    let command = str::from_utf8(command_bytes)
                        .unwrap_or("")
                        .trim_end_matches('\r')
                        .trim();
                    
                    println!("Received command: '{}'", command);
                    
                    // Process command
                    let response = process_command(command);
                    socket.write_all(response.as_bytes()).await?;
                    socket.write_all(b"\r\n> ").await?;
                    
                    // Move remaining data to beginning of buffer
                    let remaining = pos - newline_pos - 1;
                    if remaining > 0 {
                        buffer.copy_within(newline_pos + 1..pos, 0);
                    }
                    pos = remaining;
                }
                
                // Prevent buffer overflow
                if pos >= buffer.len() {
                    socket.write_all(b"ERROR: Command too long\r\n> ").await?;
                    pos = 0;
                }
            }
            Err(e) => {
                println!("Read error: {:?}", e);
                return Err(e);
            }
        }
    }
    
    Ok(())
}

#[embassy_executor::task(pool_size = WEB_TASK_POOL_SIZE)]
async fn web_task(
    id: usize,
    stack: embassy_net::Stack<'static>,
    app: &'static AppRouter<AppProps>,
    config: &'static picoserve::Config<Duration>,
    state: AppState,
) -> ! {
    let port = 80;
    let mut tcp_rx_buffer = [0; 1024];
    let mut tcp_tx_buffer = [0; 1024];
    let mut http_buffer = [0; 2048];

    picoserve::listen_and_serve_with_state(
        id,
        app,
        config,
        stack,
        port,
        &mut tcp_rx_buffer,
        &mut tcp_tx_buffer,
        &mut http_buffer,
        &state
    )
    .await
}

fn process_command(command: &str) -> &'static str {
    match command.to_uppercase().as_str() {
        "LED_ON" => {
            unsafe { LED_ON = true; }
            "OK: LED turned ON"
        }
        "LED_OFF" => {
            unsafe { LED_ON = false; }
            "OK: LED turned OFF"
        }
        "STATUS" => {
            if unsafe { LED_ON } {
                "STATUS: LED is ON"
            } else {
                "STATUS: LED is OFF"
            }
        }
        "PING" => "PONG",
        "HELP" => "Available commands: LED_ON, LED_OFF, STATUS, PING, HELP",
        "" => "ERROR: Empty command",
        _ => "ERROR: Unknown command. Type HELP for available commands",
    }
}

async fn send_message(stack: Stack<'_>) {

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];

    let mut socket = TcpSocket::new(stack,  &mut rx_buffer, &mut tx_buffer);

    socket.set_timeout(Some(embassy_time::Duration::from_secs(10)));

    let remote_endpoint = (Ipv4Addr::new(192, 168, 50, 216), 8000);
    println!("connecting...");
    let r = socket.connect(remote_endpoint).await;
    if let Err(e) = r {
        println!("connect error: {:?}", e);
        return;
    }
    println!("connected!");
    let mut buf = [0; 1024];
    loop {
        use embedded_io_async::Write;
        let r = socket
            .write_all(b"GET / HTTP/1.0\r\nHost: 192.168.50.216\r\n\r\n")
            .await;
        if let Err(e) = r {
            println!("write error: {:?}", e);
            break;
        }
        let n = match socket.read(&mut buf).await {
            Ok(0) => {
                println!("read EOF");
                break;
            }
            Ok(n) => n,
            Err(e) => {
                println!("read error: {:?}", e);
                break;
            }
        };
        println!("{}", core::str::from_utf8(&buf[..n]).unwrap());
    }
}


#[embassy_executor::task]
pub async fn connection(shared_controller: SharedControl) {
    println!("start connection task");
    
    // First, get capabilities outside the main loop
    {
        let controller = shared_controller.0.lock().await;
        println!("Device capabilities: {:?}", controller.capabilities());
    } // Lock is dropped here
    
    loop {
        match esp_wifi::wifi::wifi_state() {
            WifiState::StaConnected => {
                // Wait for disconnection event
                let mut controller = shared_controller.0.lock().await;
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                drop(controller); // Explicitly drop the lock
                Timer::after(Duration::from_millis(5000)).await;
            }
            _ => {}
        }
        
        // Check if controller is started and configure if needed
        {
            let mut controller = shared_controller.0.lock().await;
            
            if !matches!(controller.is_started(), Ok(true)) {
                let client_config = Configuration::Client(ClientConfiguration {
                    ssid: SSID.into(),
                    // password: PASSWORD.into(),
                    ..Default::default()
                });
                controller.set_configuration(&client_config).unwrap();
                println!("Starting wifi");
                controller.start_async().await.unwrap();
                println!("Wifi started!");

                println!("Scan");
                let result = controller.scan_n_async(10).await.unwrap();
                for ap in result {
                    println!("{:?}", ap);
                }
            }
        } // Lock is dropped here
        
        println!("About to connect...");
        
        // Connect in a separate scope
        {
            let mut controller = shared_controller.0.lock().await;
            match controller.connect_async().await {
                Ok(_) => println!("Wifi connected!"),
                Err(e) => {
                    println!("Failed to connect to wifi: {e:?}");
                    // Lock will be dropped at end of scope
                }
            }
        } // Lock is dropped here
        
        // Only sleep if connection failed
        if !matches!(esp_wifi::wifi::wifi_state(), WifiState::StaConnected) {
            Timer::after(Duration::from_millis(5000)).await;
        }
    }
}


#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}