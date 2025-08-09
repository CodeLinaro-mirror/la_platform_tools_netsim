# BLE Advertisers Crate

A Rust library for creating and managing Bluetooth Low Energy (BLE) legacy advertising beacons.

This crate provides a high-level API for building legacy advertising and scan response packets and for managing the scheduling of multiple beacons in a thread-safe manner. It is designed to be used as a core component in network simulators or any application that needs to model the behavior of BLE beacons.

## Key Features

- **Packet Construction**: Build custom BLE advertising and scan response packets with support for common data types like device name, TX power, manufacturer data, and GATT services.
- **Thread-Safe Scheduling**: The `Advertisers` allows you to manage multiple beacons concurrently from different async tasks or threads.
- **Event-Driven API**: Provides a simple `duration()` method to determine the wait time until the next advertising event, making it easy to integrate into an event loop.
- **CRUD Interface**: A clean, `HashMap`-like interface for creating, reading, updating, and deleting advertisers from a managed set.
- **Self-Contained**: Has minimal dependencies and defines its own necessary packet structures.

## API Overview

The library has two main public structs:

- **`Advertiser`**: Represents a single beacon. It holds the beacon's configuration (settings and data) and the pre-built, constant byte packets for advertising and scan responses.
- **`Advertisers`**: A thread-safe manager for a collection of `Advertiser` instances. It handles the scheduling logic to determine which beacon should advertise next.

## Usage Example

The main workflow involves creating one or more `Advertiser` instances, adding them to an `Advertisers`, and then using the set in a loop to get the next packet to send.

```rust
use ble_advertisers::advertiser::Advertiser;
use ble_advertisers::advertisers::Advertisers;
use ble_advertisers::advertise_settings::{AdvertiseMode, AdvertiseSettingsBuilder, TxPowerLevel};
use ble_advertisers::advertise_data::{AdvertiseData, Service};
use std::time::Duration;
use std::thread;

fn main() {
    // 1. Configure the beacon's settings and data
    let settings = AdvertiseSettingsBuilder::new()
        .mode(AdvertiseMode::new(Duration::from_millis(150)))
        .scannable() // Make it respond to scans
        .build();

    let adv_data = AdvertiseData::builder("MyBeacon".to_string(), TxPowerLevel::default())
        .include_device_name()
        .services(vec![Service {
            uuid: "180D".to_string(), // Heart Rate Service
            data: vec![0x01, 0x02],
        }])
        .build()
        .unwrap();

    // Optionally, create separate data for the scan response
    let scan_response_data = AdvertiseData::builder("MyBeacon-Scan".to_string(), TxPowerLevel::default())
        .manufacturer_data(vec![0xFF, 0x01, 0x02, 0x03]) // Example manufacturer data
        .build()
        .unwrap();

    // 2. Create an advertiser
    let address = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
    let advertiser = Advertiser::new(settings, adv_data, Some(scan_response_data), address).unwrap();

    // 3. Manage advertisers in a thread-safe set
    let set = Advertisers::new();
    set.add(1, advertiser);

    // The set can be cloned and shared across threads
    let set_clone = set.clone();
    thread::spawn(move || {
        // You can add/remove/get advertisers from another thread
        let adv2_settings = AdvertiseSettingsBuilder::new()
            .mode(AdvertiseMode::new(Duration::from_millis(500)))
            .build();
        let adv2_data = AdvertiseData::builder("AnotherBeacon".to_string(), TxPowerLevel::default()).build().unwrap();
        let adv2 = Advertiser::new(adv2_settings, adv2_data, None, [0x11; 6]).unwrap();
        set_clone.add(2, adv2);
    }).join().unwrap();


    // 4. In your main event loop, get the next event time and packet
    loop {
        if let Some((id, next_event_in)) = set.duration() {
            // Sleep until the next event is due
            thread::sleep(next_event_in);

            // Get the packet to send
            if let Some(packet) = set.packet(id) {
                println!("Sending advertising packet for beacon {}: {:?}", id, packet);

                // If the advertiser is scannable, you might also need the scan response
                if let Some(scan_response) = set.scan_response_packet(id) {
                    // Keep the scan response ready
                    // println!("Scan response for beacon {}: {:?}", id, scan_response);
                }
            }
        } else {
            // No advertisers in the set
            thread::sleep(Duration::from_secs(1));
        }
    }
}
```
