use embassy_executor::Spawner;
use embassy_net::{Stack, StackResources, Runner};
use embassy_time::{Duration, Timer};
use esp_hal::{peripherals::Peripherals, rng::Rng, timer::timg::TimerGroup, radio::RadioClockControl};
use esp_println::println;
use esp_wifi::{
    EspWifiController,
    init,
    wifi::{ClientConfiguration, Configuration, WifiController, WifiDevice, WifiEvent, WifiState},
};
use static_cell::make_static; // If used for stack resources or controller

// When you are okay with using a nightly compiler it's better to use https://docs.rs/static_cell/2.1.0/static_cell/macro.make_static.html
macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

// Define a simple error type for init_stack
#[derive(Debug)]
pub enum WifiError {
    WifiInitError, // Placeholder for more specific errors
    StackInitError,
    ControllerStartError,
}

#[embassy_executor::task]
pub async fn connection(mut controller: WifiController<'static>) {
    println!("start connection task");
    println!("Device capabilities: {:?}", controller.capabilities());
    loop {
        match esp_wifi::wifi::wifi_state() {
            WifiState::StaConnected => {
                // wait until we're no longer connected
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                Timer::after(Duration::from_millis(5000)).await
            }
            _ => {}
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: SSID.into(),
                password: PASSWORD.into(),
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            println!("Starting wifi");
            if controller.start_async().await.is_err() {
                println!("Error starting wifi controller");
                // Consider returning an error or panicking
            }
            println!("Wifi started!");

            println!("Scan");
            // Scan for networks, consider making this optional or configurable
            if let Ok(result) = controller.scan_n_async(10).await {
                 for ap in result {
                    println!("{:?}", ap);
                }
            } else {
                println!("Scan failed");
            }
        }
        println!("About to connect...");

        match controller.connect_async().await {
            Ok(_) => println!("Wifi connected!"),
            Err(e) => {
                println!("Failed to connect to wifi: {e:?}");
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

#[embassy_executor::task]
async fn net_task(runner: Runner<'static, WifiDevice<'static>>) { // Ensure WifiDevice generic matches Stack
    runner.run().await
}


pub async fn init_stack(
    spawner: Spawner,
    peripherals: Peripherals, // Pass relevant parts of peripherals
    timg0: TimerGroup<'static, esp_hal::peripherals::TIMG0>, // Pass specific timers
    rng: Rng,
    radio_clk: RadioClockControl, // Adjust type as per esp_hal version
    // wifi_peripheral: esp_hal::peripherals::WIFI, // Pass WIFI peripheral
) -> Result<(Stack<'static>, EspWifiController<'static>), WifiError> { // Adjusted return type

    let esp_wifi_ctrl: &'static EspWifiController<'static> =
        make_static!(init(timg0.timer0, rng.clone(), radio_clk).map_err(|_| WifiError::WifiInitError)?);

    let (controller, interfaces) =
        esp_wifi::wifi::new(&esp_wifi_ctrl, peripherals.WIFI).map_err(|_| WifiError::StackInitError)?;

    let wifi_interface = interfaces.sta;

    let config = embassy_net::Config::dhcpv4(Default::default());
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    let stack_resources = mk_static!(StackResources<3>, StackResources::<3>::new());
    let (stack, runner) = embassy_net::new(
        wifi_interface,
        config,
        stack_resources,
        seed,
    );

    spawner.spawn(connection(controller)).map_err(|_| WifiError::ControllerStartError)?;
    spawner.spawn(net_task(runner)).map_err(|_| WifiError::ControllerStartError)?;

    Ok((stack, esp_wifi_ctrl)) // Return both stack and controller if needed by main
}
