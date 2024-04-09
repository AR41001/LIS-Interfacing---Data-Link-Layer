STEPS TO RUN THE CODE:

    Go to the file location and open terminal
    Run the command on terminal " cargo build ". This will compile libraries and create executable files
    Run the command on terminal " cargo run "
    I tested the code on Raspberry Pi Zero 2w

LIS Interfacing - Establishment Phase

This code implements the data link layer for Laboratory Information System (LIS) interfacing.It demonstrates the exchange of data frames using control characters and checksums to ensure data integrity. 

Features:

 	1. Asynchronous Serial Communication
	2. Bi-Directional Data Transfer Protocol
	3. Frame Structure and Checksum Handling
	
How to Use:

    Compile the code using a Rust compiler.
    Run the compiled binary.
    The program will prompt you to choose between receiving or transmitting ENQ.
    Enter "1" to receive ENQ (client mode) or "2" to transmit ENQ (initiator mode).
    Follow the on-screen messages for further instructions.

Note:

    This code is for demonstration purposes only and may require further development for specific LIS implementations.
    The serial port path (/dev/ttyS0) might need to be adjusted based on your system configuration.

Code Breakdown:
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

Further Development:

    Integrate this code with higher-level LIS communication protocols.
    Implement error handling for unexpected data or communication failures.

I hope this readme provides a clear explanation of the code and its functionality.
