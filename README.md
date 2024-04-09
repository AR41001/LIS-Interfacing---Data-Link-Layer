LIS Interfacing - Data Link Layer

Overview

This code is a simulation of a communication protocol between two devices over a serial port. It demonstrates the exchange of data frames using control characters and checksums to ensure data integrity.
Configure the serial port path and baud rate in the code:
    Replace "/dev/ttyS0" with the correct serial port path and 9600 with the desired baud rate.

Description

    The code simulates a client-server scenario where one device acts as the sender and the other as the receiver.
    Control characters such as ENQ, ACK, NAK, STX, ETX, ETB, CR, LF, and EOT are utilized for framing and communication.
    Checksums are calculated and verified to ensure data integrity.
    The program demonstrates establishment phase, transfer phase, and termination phase of communication.

Functions

    1.receiver_state_awake: Initiates the receiving state by sending ACK upon receiving ENQ.
    2.waiting_for_frame: Waits for STX to enter the transfer phase.
    3.frame_received: Receives and processes data frames, calculates checksums, and verifies integrity.
    4.data_to_send: Initiates the establishment phase by sending ENQ and handling responses.
    5.next_frame_setup: Handles the sending of data frames, dividing into ETX or ETB frames based on size.
    6.checksum: Calculates checksum for data frames.
    7.checksum_match: Verifies checksum received from the sender.
    8.frame_ready: Sends data frames serially to the receiver.
    9.termination_phase: Sends EOT to signal end of transmission.
    10.reset_states: Resets various states and counters for the next communication cycle.

Notes

    This code is a simulation and may need adjustments for actual hardware configurations.
    Customize serial port settings and timeouts as per your requirements.
    Follow appropriate error handling practices for production use.
