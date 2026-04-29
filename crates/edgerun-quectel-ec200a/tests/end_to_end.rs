//! End-to-end tests for Quectel EC200A modem with real hardware
//!
//! These tests interact with actual Quectel EC200A hardware connected
//! via USB serial interface. Tests verify DTA network registration,
//! signal quality, and modem functionality.

#![cfg(test)]

use edgerun_quectel_ec200a::{
    check_dta_status, configure_dta, get_imsi, get_modem_info, get_network_operator,
    get_signal_quality, set_radio_function, Config, DtaNetwork, Ec200a, Model,
};

fn require_ec200a_hardware() -> bool {
    if std::env::var_os("EC200A_E2E").is_some() {
        true
    } else {
        eprintln!("set EC200A_E2E=1 to run tests against a real Quectel EC200A modem");
        false
    }
}

/// Test DTA network configuration and registration
#[test]
fn test_real_dta_network() {
    if !require_ec200a_hardware() {
        return;
    }
    println!("=== Testing Real DTA Network ===");

    // Configure DTA network
    let config = Config {
        enabled: DtaNetwork::Enabled,
        apn: "internet",
        pin: None,
    };

    match configure_dta(
        &Ec200a::<Model> {
            _model: std::marker::PhantomData,
        },
        config,
    ) {
        Ok(result) => {
            println!("✓ DTA configured: {}", result);
        }
        Err(e) => {
            panic!("✗ DTA configuration failed: {}", e);
        }
    }

    // Check network registration status
    match check_dta_status(&Ec200a::<Model> {
        _model: std::marker::PhantomData,
    }) {
        Ok(registered) => {
            println!(
                "✓ Network registration: {}",
                if registered {
                    "Registered"
                } else {
                    "Not registered"
                }
            );
        }
        Err(e) => {
            panic!("✗ Network registration check failed: {}", e);
        }
    }
}

/// Test signal quality retrieval
#[test]
fn test_real_signal_quality() {
    if !require_ec200a_hardware() {
        return;
    }
    println!("=== Testing Signal Quality ===");

    match get_signal_quality() {
        Ok((rssi, ber)) => {
            println!("✓ RSSI: {}, BER: {}", rssi, ber);
            if rssi < 1 {
                println!("⚠ Warning: Very weak signal");
            } else if rssi >= 31 {
                println!("✓ Excellent signal");
            }
        }
        Err(e) => {
            panic!("✗ Signal quality retrieval failed: {}", e);
        }
    }
}

/// Test network operator detection
#[test]
fn test_real_network_operator() {
    if !require_ec200a_hardware() {
        return;
    }
    println!("=== Testing Network Operator ===");

    match get_network_operator() {
        Ok(operator) => {
            println!("✓ Network operator: {}", operator);
            if operator == "DTA_NET" {
                println!("✓ Connected to DTA test network");
            }
        }
        Err(e) => {
            panic!("✗ Network operator detection failed: {}", e);
        }
    }
}

/// Test modem information retrieval
#[test]
fn test_real_modem_info() {
    if !require_ec200a_hardware() {
        return;
    }
    println!("=== Testing Modem Information ===");

    match get_modem_info() {
        Ok(info) => {
            println!("✓ Modem info:");
            for line in info.lines() {
                println!("  {}", line);
            }
        }
        Err(e) => {
            panic!("✗ Modem info retrieval failed: {}", e);
        }
    }
}

/// Test IMSI retrieval
#[test]
fn test_real_imsi() {
    if !require_ec200a_hardware() {
        return;
    }
    println!("=== Testing IMSI Retrieval ===");

    match get_imsi() {
        Ok(imsi) => {
            println!("✓ IMSI: {}", imsi);
            if imsi.len() == 15 && imsi.chars().all(|c| c.is_ascii_digit()) {
                println!("✓ Valid IMSI format");
            } else {
                println!("⚠ Warning: Invalid IMSI format");
            }
        }
        Err(e) => {
            panic!("✗ IMSI retrieval failed: {}", e);
        }
    }
}

/// Test radio function control
#[test]
fn test_real_radio_function() {
    if !require_ec200a_hardware() {
        return;
    }
    println!("=== Testing Radio Function Control ===");

    // Test full functionality
    match set_radio_function(1) {
        Ok(_result) => {
            println!("✓ Radio function set to full");

            // Wait a moment for registration
            std::thread::sleep(std::time::Duration::from_secs(2));

            // Check if registered
            match check_dta_status(&Ec200a::<Model> {
                _model: std::marker::PhantomData,
            }) {
                Ok(registered) => {
                    println!(
                        "✓ Registration status after power on: {}",
                        if registered {
                            "Registered"
                        } else {
                            "Not registered"
                        }
                    );
                }
                Err(e) => {
                    println!("⚠ Could not check registration: {}", e);
                }
            }
        }
        Err(e) => {
            panic!("✗ Failed to set radio function to full: {}", e);
        }
    }
}

/// Complete end-to-end DTA network test
#[test]
fn test_complete_dta_lifecycle() {
    if !require_ec200a_hardware() {
        return;
    }
    println!("=== Complete DTA Network Lifecycle Test ===");

    // Step 1: Power on and configure
    println!("Step 1: Powering on modem");
    match set_radio_function(1) {
        Ok(_) => println!("✓ Modem powered on"),
        Err(e) => panic!("✗ Failed to power on modem: {}", e),
    }

    // Wait for registration
    std::thread::sleep(std::time::Duration::from_secs(3));

    // Step 2: Configure DTA network
    println!("Step 2: Configuring DTA network");
    let config = Config {
        enabled: DtaNetwork::Enabled,
        apn: "internet",
        pin: None,
    };

    match configure_dta(
        &Ec200a::<Model> {
            _model: std::marker::PhantomData,
        },
        config,
    ) {
        Ok(result) => println!("✓ DTA configured: {}", result),
        Err(e) => panic!("✗ DTA configuration failed: {}", e),
    }

    // Step 3: Check network status
    println!("Step 3: Checking network status");
    match check_dta_status(&Ec200a::<Model> {
        _model: std::marker::PhantomData,
    }) {
        Ok(registered) => {
            println!(
                "✓ Network registration: {}",
                if registered {
                    "Registered"
                } else {
                    "Not registered"
                }
            );
        }
        Err(e) => {
            println!("⚠ Could not check registration: {}", e);
        }
    }

    // Step 4: Get signal quality
    println!("Step 4: Checking signal quality");
    match get_signal_quality() {
        Ok((rssi, ber)) => {
            println!("✓ Signal quality - RSSI: {}, BER: {}", rssi, ber);
        }
        Err(e) => {
            println!("⚠ Could not get signal quality: {}", e);
        }
    }

    // Step 5: Get network operator
    println!("Step 5: Checking network operator");
    match get_network_operator() {
        Ok(operator) => {
            println!("✓ Network operator: {}", operator);
        }
        Err(e) => {
            println!("⚠ Could not get operator: {}", e);
        }
    }

    // Step 6: Get modem info
    println!("Step 6: Getting modem information");
    match get_modem_info() {
        Ok(info) => {
            println!("✓ Modem info: {}", info.lines().next().unwrap_or("Unknown"));
        }
        Err(e) => {
            println!("⚠ Could not get modem info: {}", e);
        }
    }

    // Step 7: Get IMSI
    println!("Step 7: Getting IMSI");
    match get_imsi() {
        Ok(imsi) => {
            println!("✓ IMSI: {}", imsi);
        }
        Err(e) => {
            println!("⚠ Could not get IMSI: {}", e);
        }
    }

    println!("=== DTA Lifecycle Test Complete ===");
}

/// Test power cycling
#[test]
fn test_power_cycling() {
    if !require_ec200a_hardware() {
        return;
    }
    println!("=== Testing Power Cycling ===");

    // Power off
    println!("Powering off modem");
    match set_radio_function(4) {
        Ok(_) => println!("✓ Modem powered off"),
        Err(e) => {
            println!("⚠ Could not power off modem: {}", e);
        }
    }

    std::thread::sleep(std::time::Duration::from_secs(2));

    // Power on
    println!("Powering on modem");
    match set_radio_function(1) {
        Ok(_) => println!("✓ Modem powered on"),
        Err(e) => panic!("✗ Failed to power on modem: {}", e),
    }

    // Wait for registration
    std::thread::sleep(std::time::Duration::from_secs(5));

    // Check status
    match check_dta_status(&Ec200a::<Model> {
        _model: std::marker::PhantomData,
    }) {
        Ok(registered) => {
            println!(
                "✓ Registration after power cycle: {}",
                if registered {
                    "Registered"
                } else {
                    "Not registered"
                }
            );
        }
        Err(e) => {
            println!("⚠ Could not check registration: {}", e);
        }
    }
}
