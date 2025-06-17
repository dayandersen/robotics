use esp_hal::gpio::{Output, Gpio18};

static mut LED_ON: bool = false;

pub struct LedControl {
    pin: Output<'static, Gpio18>,
}

// Public static functions to modify the global LED_ON state.
// These are unsafe due to static mutable access.
pub fn set_led_state(state: bool) {
    unsafe { LED_ON = state; }
}

pub fn get_led_state() -> bool {
    unsafe { LED_ON }
}

impl LedControl {
    pub fn new(pin: Output<'static, Gpio18>) -> Self {
        LedControl { pin }
    }

    // Instance methods set_on/set_off now call the static function
    pub fn set_on(&mut self) { // Keep this if direct control over an instance is also needed
        set_led_state(true);
    }

    pub fn set_off(&mut self) { // Keep this
        set_led_state(false);
    }

    // is_on now uses the static getter
    pub fn is_on(&self) -> bool { // Keep this
        get_led_state()
    }

    pub fn update_hw(&mut self) {
        if unsafe { LED_ON } {
            self.pin.set_high().unwrap();
        } else {
            self.pin.set_low().unwrap();
        }
    }
}
