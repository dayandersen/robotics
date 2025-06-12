# ESP32 Rust Development Setup

## Core Dependencies

### Build Tools
```bash
# macOS
brew install cmake ninja dfu-util ccache git
# Optional: pkg-config for some crates
brew install pkg-config

# Linux (Ubuntu/Debian)
sudo apt-get install git cmake ninja-build ccache dfu-util libusb-1.0-0 \
    libssl-dev libffi-dev python3 python3-pip python3-venv

# Linux (Arch)
sudo pacman -S git cmake ninja ccache dfu-util libusb python-pip
```

### USB Permissions (Linux)
```bash
# Add user to dialout group for device access
sudo usermod -a -G dialout $USER
# Log out and back in for changes to take effect
```

### ESP-IDF Installation
```bash
mkdir -p ~/esp
cd ~/esp
git clone -b v5.4.1 --recursive https://github.com/espressif/esp-idf.git
```

### Rust Toolchain
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
cargo install espup
espup install
```

## Environment Configuration

Source ESP environment (required for each session):
```bash
. $HOME/export-esp.sh
```

To make permanent, add to shell profile:
```bash
echo '. $HOME/export-esp.sh' >> ~/.zshrc
# or ~/.bashrc
```

## ESP32 Device Architecture Reference

| Device/Chip | Architecture | Core Count | Max Frequency | Notes |
|-------------|--------------|------------|---------------|-------|
| **ESP32 SoCs** |
| ESP32 (original) | Xtensa LX6 | Dual | 240 MHz | Classic + BLE 4.2 |
| ESP32-D0WDQ6 | Xtensa LX6 | Dual | 240 MHz | Specific chip variant |
| ESP32-D0WD | Xtensa LX6 | Dual | 240 MHz | Smaller footprint variant |
| ESP32-S0WD | Xtensa LX6 | Single | 160 MHz | Single-core variant |
| ESP32-solo1 | Xtensa LX6 | Single | 160 MHz | Commercial single-core |
| ESP32-S2 | Xtensa LX7 | Single | 240 MHz | USB OTG, no Bluetooth |
| ESP32-S3 | Xtensa LX7 | Dual | 240 MHz | AI acceleration, BLE 5.0 |
| **ESP32-C Series (RISC-V)** |
| ESP8684/ESP32-C2 | RISC-V | Single | 120 MHz | Low-cost, embedded flash |
| ESP8685/ESP32-C3 | RISC-V | Single | 160 MHz | Security-focused, BLE 5.0 |
| ESP32-C5 | RISC-V | Single | 160 MHz | Dual-band WiFi 6 (2.4+5 GHz) |
| ESP32-C6 | RISC-V | Single | 160 MHz | WiFi 6, Thread/Zigbee, BLE 5.3 |
| ESP32-C61 | RISC-V | Single | 160 MHz | C6 variant with PSRAM |
| **ESP32-H Series (RISC-V)** |
| ESP32-H2 | RISC-V | Single | 96 MHz | Thread/Zigbee only, no WiFi |
| **ESP32-P Series (RISC-V)** |
| ESP32-P4 | RISC-V | Dual | 400 MHz | Edge AI, H.264, no WiFi/BLE |
| **Common Modules** |
| ESP32-WROOM-32 | Xtensa LX6 | Dual | 240 MHz | Based on ESP32-D0WDQ6 |
| ESP32-WROOM-32D | Xtensa LX6 | Dual | 240 MHz | Based on ESP32-D0WD |
| ESP32-WROOM-32U | Xtensa LX6 | Dual | 240 MHz | Smallest WROOM variant |
| ESP32-WROVER-B | Xtensa LX6 | Dual | 240 MHz | With 8MB PSRAM |
| ESP32-PICO-D4 | Xtensa LX6 | Dual | 240 MHz | SiP with 4MB flash |
| ESP32-PICO-V3 | Xtensa LX6 | Dual | 240 MHz | Updated PICO variant |
| ESP32-S2-WROOM | Xtensa LX7 | Single | 240 MHz | S2-based module |
| ESP32-S2-WROVER | Xtensa LX7 | Single | 240 MHz | S2 with PSRAM |
| ESP32-S3-WROOM-1 | Xtensa LX7 | Dual | 240 MHz | S3-based module |
| ESP32-S3-PICO-1 | Xtensa LX7 | Dual | 240 MHz | S3 SiP with PSRAM |
| ESP32-C3-WROOM-02 | RISC-V | Single | 160 MHz | C3-based module |
| ESP32-C3-MINI-1 | RISC-V | Single | 160 MHz | Compact C3 module |
| ESP32-C6-WROOM-1 | RISC-V | Single | 160 MHz | C6-based module |

### Target Selection for espup
```bash
# Install specific architectures
espup install -t esp32c3,esp32c6,esp32h2    # RISC-V only
espup install -t esp32,esp32s2,esp32s3      # Xtensa only
espup install -t all                        # All architectures
```

## Verification

```bash
rustc --version
cargo --version

# For Xtensa targets only
which xtensa-esp32-elf-gcc

# For RISC-V targets
which riscv32-esp-elf-gcc
```

Your environment is ready when relevant commands return valid output for your target architecture.