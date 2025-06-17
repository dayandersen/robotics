// For now, we'll assume led_control functions will be available globally or passed.
// This will likely cause errors until led_control is integrated.
// Consider a more robust way to handle state, e.g., passing a mutable reference
// to LedControl or using a shared state mechanism if this were a real concurrent scenario.

// Placeholder for where LED control functions would be called
// For this step, we are just moving the existing logic which uses a global static.
// The actual calls to a new `LedControl` instance will be done in a later step
// when `main.rs` is refactored.

// This static variable is problematic and should be managed by LedControl.
// We are moving it here temporarily as part of the refactoring process.
// It will be removed once LedControl is fully integrated.
// static mut LED_ON: bool = false; // Removed, state is now managed in led_control

// Assuming led_control is accessible as `crate::led_control`
use crate::led_control;

pub fn process_command(command: &str) -> &'static str {
    match command.to_uppercase().as_str() {
        "LED_ON" => {
            led_control::set_led_state(true);
            "OK: LED turned ON"
        }
        "LED_OFF" => {
            led_control::set_led_state(false);
            "OK: LED turned OFF"
        }
        "STATUS" => {
            if led_control::get_led_state() {
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
