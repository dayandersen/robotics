use embassy_net::{tcp::TcpSocket, Stack, IpListenEndpoint};
use embassy_time::{Duration, Timer};
use esp_println::println;
use static_cell::make_static; // May not be needed if stack is passed in

use crate::command_handler::process_command; // Assuming command_handler is in lib.rs or main.rs declares it

// If WifiDevice is needed, it must be correctly parameterized or passed.
// use esp_wifi::wifi::WifiDevice; // This might be needed depending on Stack's definition

#[embassy_executor::task]
pub async fn recv_message(stack: Stack<'static>) { // Consider if WifiDevice generic is needed for Stack
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
                Timer::after(Duration::from_millis(1000)).await; // Consider making delay configurable
            }
        }

        socket.close();
        Timer::after(Duration::from_millis(100)).await; // Consider making delay configurable
    }
}

async fn handle_client(socket: &mut TcpSocket<'_>) -> Result<(), embassy_net::tcp::Error> {
    use embedded_io_async::{Read, Write};
    use core::str; // Required for str::from_utf8

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

                if let Some(newline_pos) = buffer[..pos].iter().position(|&b| b == b'\n') {
                    let command_bytes = &buffer[..newline_pos];
                    let command = str::from_utf8(command_bytes)
                        .unwrap_or("")
                        .trim_end_matches('\r')
                        .trim();

                    println!("Received command: '{}'", command);

                    let response = process_command(command); // Calls into command_handler
                    socket.write_all(response.as_bytes()).await?;
                    socket.write_all(b"\r\n> ").await?;

                    let remaining = pos - newline_pos - 1;
                    if remaining > 0 {
                        buffer.copy_within(newline_pos + 1..pos, 0);
                    }
                    pos = remaining;
                }

                if pos >= buffer.len() {
                    socket.write_all(b"ERROR: Command too long\r\n> ").await?;
                    pos = 0; // Reset buffer position
                }
            }
            Err(e) => {
                println!("Read error: {:?}", e);
                return Err(e); // Propagate TCP errors
            }
        }
    }

    Ok(())
}
