LIS Interfacing - Data Link Layer

Overview

This code is a simulation of a communication protocol between two devices over a serial port. It demonstrates the exchange of data frames using control characters and checksums to ensure data integrity.
Requirements

    Rust programming language environment
    Dependencies: rppal, serialport

Usage

    Clone the repository.
    Ensure Rust is installed on your system.
    Install dependencies by adding them to your Cargo.toml:

    toml

    [dependencies]
    libc = "0.2.1"
    termios = "0.2.2"
    ioctl-rs = "0.1.5"
    serialport = "4.2.2"  # Add the version of the serialport crate here
    clap = { version = "3.1.6", features = ["derive"] }
    rust_gpiozero = "^0.2"
    rppal = "0.11.2"
    embedded-graphics = "0.6.0"
    linux-embedded-hal = "0.3.0"

    [dev-dependencies]
    assert_hex = "0.2.2"

Configure the serial port path and baud rate in the code:
    Replace "/dev/ttyS0" with the correct serial port path and 9600 with the desired baud rate.

Description

    The code simulates a client-server scenario where one device acts as the sender and the other as the receiver.
    Control characters such as ENQ, ACK, NAK, STX, ETX, ETB, CR, LF, and EOT are utilized for framing and communication.
    Checksums are calculated and verified to ensure data integrity.
    The program demonstrates establishment phase, transfer phase, and termination phase of communication.

Functions

    receiver_state_awake: Initiates the receiving state by sending ACK upon receiving ENQ.
    waiting_for_frame: Waits for STX to enter the transfer phase.
    frame_received: Receives and processes data frames, calculates checksums, and verifies integrity.
    data_to_send: Initiates the establishment phase by sending ENQ and handling responses.
    next_frame_setup: Handles the sending of data frames, dividing into ETX or ETB frames based on size.
    checksum: Calculates checksum for data frames.
    checksum_match: Verifies checksum received from the sender.
    frame_ready: Sends data frames serially to the receiver.
    termination_phase: Sends EOT to signal end of transmission.
    reset_states: Resets various states and counters for the next communication cycle.

Notes

    This code is a simulation and may need adjustments for actual hardware configurations.
    Customize serial port settings and timeouts as per your requirements.
    Follow appropriate error handling practices for production use.
