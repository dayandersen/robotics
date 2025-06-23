
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