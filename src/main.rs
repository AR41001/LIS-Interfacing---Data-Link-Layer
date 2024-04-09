#![allow(unused)]
use rppal::gpio::{Gpio, OutputPin};
use serialport::SerialPort;
use std::collections::VecDeque;
use std::io;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const ENQ: &[u8] = &[0x05]; // Enquiry
const ACK: &[u8] = &[0x06]; // Acknowledge
const NAK: &[u8] = &[0x15]; // Not Acknowledged
const STX: u8 = 0x02;       // Control Character for initiating transfer phase
const ETX: u8 = 0x03;       // Control Character for identifying end of end-frame
const ETB: u8 = 0x17;       // Control Character for identifying end of intermediate-frame
const CR: u8 = 0x0D;        // Control Character for identifying second last character of any frame
const LF: u8 = 0x0A;        // Control Character for identifying last character of any frame
const EOT: u8 = 0x04;       // Control Character for identifying end of transmission. When this is received both devices go back in idle state

fn main() {
    let mut message_to_send = "Anything u wanna send";

    let mut frame_no: u8 = 48;  // frame number starts with 0 always. 48 is the ASCII equivalent
    let mut enq_attempts = 0;   
    let mut client_buffer = [0u8; 1];
    let mut port = serialport::new("/dev/ttyS0", 9600)
        .timeout(Duration::from_secs(2)) // was getting timeout errors so changed to 2 secs
        .open()
        .expect("Failed to open port");

    let _establish_connection_to_send_new_frame = true;     
    let mut more_frames_left_to_read;                   // checks if frames are left to read
    let mut go_for_termination = false;                 // checks whether we go for termination phase
    let mut go_for_contention = false;                  // whether contention is reached
    let mut received_stx;                               // check for STX determines whether we have entered in transpher phase
    let mut tries_left_for_ack = false;                 // these tries are in place if one of the device disconnects midway. 10 tries in total
    let mut connection_established = false;             // checks whether connection is established
    let mut was_data_sent_successfully;                 // checks whether data is sent successfully, important for the flow of code
    let mut contention_timer = Instant::now();       

    let mut start_time = Instant::now();

    loop {
        let response = port.read(&mut client_buffer).is_err();
        println!("Welcome to the CLIENT side, do you want to transmit or receieve ENQ");
        println!("Choose an option:");
        println!("1. Receive ENQ");
        println!("2. Transmit ENQ");
        let mut choice = String::new();
        io::stdin()
            .read_line(&mut choice)
            .expect("Failed to read line");
        let choice: u32 = match choice.trim().parse() {
            Ok(num) => num,
            Err(_) => {
                println!("Invalid input, please enter 1 or 2.");
                continue;
            }
        };

        match choice {
            1 => {
                println!("Entered sending side");
                println!("tries left for ack {:?}", tries_left_for_ack);
                while !tries_left_for_ack && enq_attempts <= 10 {
                    (
                        frame_no,
                        go_for_termination,
                        go_for_contention,
                        tries_left_for_ack,
                        connection_established,
                    ) = data_to_send(&mut port, frame_no, message_to_send, message_to_send.len());
                    enq_attempts += 1;
                }
                if go_for_termination {
                    termination_phase(&mut port);
                } else if go_for_contention == true {
                    contention_timer = Instant::now(); 
                    println!("Contention reached, according to ur machine's protocol");
                } else if connection_established {
                    (frame_no, was_data_sent_successfully) = next_frame_setup(
                        &mut port,
                        frame_no,
                        message_to_send,
                        message_to_send.len(),
                    );
                    if !was_data_sent_successfully {
                        println!("Problem in sending data, going to idle state");
                        continue;
                    } else if was_data_sent_successfully {
                        println!("Sent everything now resetting states and going to idle state");
                        reset_states(
                            &mut enq_attempts,
                            &mut tries_left_for_ack,
                            &mut go_for_termination,
                            &mut go_for_contention,
                            &mut connection_established,
                            &mut frame_no,
                        );
                    }

                } else {
                    continue;
                }
            }

            2 => {
                println!("Entered receiver's loop");
                println!("The frame number is {}", frame_no);
                let buffer_byte = client_buffer[0];
                if client_buffer == ENQ {
                    go_for_contention = false;
                    println!("The queue is empty");
                    receiver_state_awake(&mut port, frame_no);
                    println!("the frame number before increment {}", frame_no);
                    loop {
                        (frame_no, received_stx) = waiting_for_frame(&mut port, frame_no);
                        if received_stx == false {
                            println!("Did not receive stx, going to idle state");
                            break;
                        } else {
                            println!(" frame number after increment{}", frame_no);
                            (more_frames_left_to_read, frame_no) =
                                frame_received(&mut port, frame_no);
                            if !more_frames_left_to_read {
                                println!("The frame number at the end is {}", frame_no);
                                println!("Completed one complete cycle");
                                break;
                            } else {
                                continue;
                            }
                        }
                    }
                }
            }
            0_u32 | 3_u32..=u32::MAX => todo!(),
        }
    }
}
/// In this function the device is waiting to receive an ENQ from the sender and sends an ACK when it does. Now since it is a simulation I have commented out the sending NAK condition which applies if we dont receive an ENQ. When we do receive an ENQ we move forward to the waiting_for_frame()
fn receiver_state_awake(port: &mut Box<dyn SerialPort>, _frame_no: u8) {
    port.write_all(ACK).expect("Failed to write [ACK]"); //Here we can choose to send ACK or NAK but for testing purposes we will respond with ACK
    port.flush().expect("Failed to flush");
    println!("Sent [ACK].");

    return;
}

/// Here we wait to receive STX which is an indication from the sender that its gonna send a frame so we wait to receive that and once we do, we receive a Frame# which is matched in the frame_number_waiting_state()
fn waiting_for_frame(port: &mut Box<dyn SerialPort>, mut frame_no: u8) -> (u8, bool) {
    let mut received_stx = false;

    println!("We are currently waiting for STX to enter Transfer Phase");

    let mut transmission_buffer = [0u8];

    match port.read(&mut transmission_buffer) {
        Ok(_) => {
            let stx_byte = transmission_buffer[0];
            println!("Received byte: {}", stx_byte);
            if stx_byte == STX {
                println!("Received STX");
                if frame_no == 55 {                     // when frame number reaches 7, it is reset to 0. 47 is incremented which makes it 0
                    frame_no = 47;
                }
                frame_no += 1;
                received_stx = true;
            } else {

            }
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::TimedOut {
                println!("Timeout occurred. No more data to read.");
            } else {
                eprintln!("Error reading from port: {:?}", e);
            }
            // Returning frame_no in case of an error
        }
    }
    return (frame_no, received_stx);
}

///Here we check the frame number received. If they dont match we send NAK after which the sender has to send the frame again, if they match we move on

/// Here the "message" of the frame is received and stored. Moreover, its checksum is also calculated and checked
fn frame_received(port: &mut Box<dyn SerialPort>, mut frame: u8) -> (bool, u8) {
    println!("Now we are waiting to receieve the message");
    let mut data_buffer = String::from("");
    let mut is_last_frame = false;
    let timeout_for_ack = 30;
    let mut received_all_data = false; // let mut did_frames_match = false;
                                       // This loop is basically receiving data continously which is being stored in the data_buffer because currently we know that it is sending the "message". In the actual application we
                                       // will have to make it more generic

    // The if condition for ETX || ETB tells us that ok, all the "message" is received so we move on to the part of calculating the checksum
    let ack_time = Instant::now();
    while Instant::now() - ack_time <= Duration::from_secs(timeout_for_ack) && !received_all_data {
        let mut byte_buffer = [0u8];
        match port.read(&mut byte_buffer) {
            Ok(_) => {
                let frame_byte = byte_buffer[0];

                if frame_byte == frame {
                    println!("The frame numbers match");
                    port.write(ACK).expect("Write failed");
                    port.flush().expect("Failed to flush");

                    let mut transmission_buffer = [0u8];
                    loop {
                        match port.read(&mut transmission_buffer) {
                            Ok(_) => {
                                let byte = transmission_buffer[0];
 
                                if byte == ETX || byte == ETB {
                                    println!("We have received ETX/ETB that means we have receieved all the data");
                                    println!("Received data: {}", data_buffer);
                                    let (sum, cs1, cs2) =
                                        checksum(&data_buffer, frame.into(), byte);
                                    if byte == ETX {
                                        is_last_frame = true;                       // if we receieve ETX that means the specific message no matter how long it was, is now received
                                    }                                               // otherwise we keep receiving, means we get ETB
                                    println!("The final sum is: {}", sum);
                                    println!("CS1 is: {}", cs1);
                                    println!("CS2 is: {}", cs2);

                                    checksum_match(port, cs1, cs2);
                                } else if byte == CR {
                                    println!("Received CR");
                                } else if byte == LF {
                                    println!("Received LF");
                                    received_all_data = true;
                                    break;
                                } else {
                                    data_buffer.push(byte as char);
                                }
                            }
                            Err(e) => {
                                if e.kind() == std::io::ErrorKind::TimedOut {
                                    println!("Timeout occurred. No more data to read in the transmission buffer");
                                } else {
                                    eprintln!("Error reading from port in the transmission buffer loop: {:?}", e);
                                }
                            }
                        }
                    }
                } else if received_all_data == true {
                    break;
                } else {
                    println!("The frame numbers dont match, Send NAK");
                    port.write(NAK).expect("Write failed");
                   
                }
            }

            Err(e) => {
                if e.kind() == std::io::ErrorKind::TimedOut {
                    println!("Timeout occurred. No more data to read.");
                } else {
                    eprintln!("Error reading from port: {:?}", e);
                }
                break;
            }
        }
    }
    if is_last_frame == true {
        println!("Exited the loop");
        println!("Received the last frame, sending EOT to inform that its end of transmission");
        port.write(ACK).expect("Write failed");
        frame = 47; 
        return (false, frame);
    } else {
        println!("Received frame, waiting for more frames");
        port.write(ACK).expect("Write failed");
        return (true, frame);
    }
}
/// This is the first function for the sender side, here we send ENQ and then wait for the response. According to response we either move forward or send ENQ again
fn data_to_send(
    port: &mut Box<dyn SerialPort>,
    frame_no: u8,
    _received_message_from_server: &str,
    _received_message_from_server_size: usize,
) -> (u8, bool, bool, bool, bool) {
    // Here we are initiating the establishment phase by sending an ENQ to the machine

    println!("We are in the establishment phase");

    let mut contention_reached = false;
    let mut is_connection_established = false;
    let mut go_for_termination = false;
    let mut retries_left = true;
    let timeout_for_ack = 15;

    let enq_time = Instant::now(); //setting timer to 15 seconds
    port.write_all(ENQ).expect("Failed to write [ENQ]");
    port.flush().expect("Failed to flush");

    while !is_connection_established
        && Instant::now() - enq_time <= Duration::from_secs(timeout_for_ack)
    {
        // Check if a response is received, below are the conditions for all the responses possible in this phase.
        if port.bytes_to_read().expect("Failed to get bytes to read") > 0 {
            let mut client_response = [0u8; 1];
            port.read_exact(&mut client_response)
                .expect("Failed to read response");

            if client_response == ACK {
                println!("ACK received. Establishment phase complete.");
                is_connection_established = true;
                retries_left = true;
                break;
            } else if client_response == NAK {
                println!("NAK received. Retrying in 10 seconds.");
                retries_left = false;
                break;
            } else if client_response == ENQ {
                println!("Reached contention because of ENQ");
                contention_reached = true;
                retries_left = true;
                break;
            }
        }
    }
    if is_connection_established {
        println!("Received ack");
        (
            frame_no,
            go_for_termination,
            contention_reached,
            retries_left,
            is_connection_established,
        )
    } else if contention_reached {
        println!("Reached contention");
        (
            frame_no,
            go_for_termination,
            contention_reached,
            retries_left,
            is_connection_established,
        )
    } else if !retries_left {
        println!("Tried 10 times to establish connection and failed, Communication is OFF");
        (
            frame_no,
            go_for_termination,
            contention_reached,
            retries_left,
            is_connection_established,
        )
    } else if retries_left {
        println!("Received NAK, retrying again until I reach 10 tries");
        (
            frame_no,
            go_for_termination,
            contention_reached,
            retries_left,
            is_connection_established,
        )
    } else {
        println!("Timeout occured");
        go_for_termination = true;
        (
            frame_no,
            go_for_termination,
            contention_reached,
            retries_left,
            is_connection_established,
        )
    }
}

/// In this function we check whether the message is greater than 240 or not, based on which we decide wether we use ETX or ETB as control characters
fn next_frame_setup(
    port: &mut Box<dyn SerialPort>,
    mut frame_no: u8,
    received_message_from_server: &str,
    received_message_from_server_size: usize,
) -> (u8, bool) {
    let mut latest_frame_no = 0;
    let mut was_message_greater_than_twoforty = false;
    let mut was_data_sent_successfully_in_breakdown = true;
    let mut was_data_sent_successfully_in_etx = true;
    println!("Entered transfer phase to send data to client");
    if received_message_from_server_size <= 240 {
        let message_size = received_message_from_server_size;
        println!("The message size is less than 240");
        println!("The frame number in next_frame_setup : {}", frame_no);
        (frame_no, was_data_sent_successfully_in_etx) =
            send_etx_frame(port, received_message_from_server, frame_no, message_size);
    } else {
        let message_size = received_message_from_server_size;
        println!("{}", message_size);
        println!("The message size is greater than 240");
        (
            latest_frame_no,
            was_message_greater_than_twoforty,
            was_data_sent_successfully_in_breakdown,
        ) = message_breakdown(port, received_message_from_server, message_size, frame_no);
    }
    if !was_data_sent_successfully_in_breakdown {
        println!("Data was not successfully sent");
        return (frame_no, was_data_sent_successfully_in_breakdown);
    } else if !was_data_sent_successfully_in_etx {
        println!("Data was not successfully sent");
        return (frame_no, was_data_sent_successfully_in_etx);
    } else {
        println!("Sent everything!");
        println!("------------------------------------------------------------------------------------------------------------------------------------------------------------");
        if was_message_greater_than_twoforty {
            return (latest_frame_no, was_data_sent_successfully_in_breakdown);
        } else {
            return (frame_no, was_data_sent_successfully_in_etx);
        }
    }
}
/// This function is for the ETX frame. Basically the frames having message size less than 240 are called ETX frames and are sent in one go
fn send_etx_frame(
    port: &mut Box<dyn SerialPort>,
    message_to_send: &str,
    mut frame_no: u8,
    _message_length: usize,
) -> (u8, bool) {
    let was_data_sent_successfully;
    println!("Entered the send_etx_frame() function");
    if frame_no == 55 {
        frame_no = 47;
    }
    frame_no = frame_no + 1;

    let (sum, cs1, cs2) = checksum(message_to_send, frame_no.into(), ETX);
    println!("{}", sum);
    was_data_sent_successfully = frame_ready(port, message_to_send, frame_no, ETX, cs1, cs2);
    if was_data_sent_successfully {
        println!("Testing something for frame number");
        // frame_no = 47;
        return (frame_no, was_data_sent_successfully);
    } else {
        println!("Problem in sending data");
        return (frame_no, was_data_sent_successfully);
    }
}

/// This message is for the ETB frames. Any frame greater than 240 is divided into slices of 240 and sent one by one. *IMPORTANT POINT* the last slice is considered ETX and is send from the ETX function
fn message_breakdown(
    port: &mut Box<dyn SerialPort>,
    received_message: &str,
    size: usize,
    mut frame_no: u8,
) -> (u8, bool, bool) {
    // println!("{}",size as u8);
    let mut was_data_sent_successfully: bool = false;
    println!("Entered the message_breakdown() function");
    let data_size: u16 = 240;
    let quotient: f32 = (size as f32 / data_size as f32) as f32;
    // println!("{}",quotient);
    let remainder: u16 = (size as u16 % data_size as u16) as u16;
    // println!("{}",remainder);
    // let frames: f32 = (size as u8/ data_size) as f32;
    // println!("{}", frames);
    let frame = quotient.ceil() as u16;
    println!("{}", frame);
    let intermediate_message = received_message;
    // println!("{}",intermediate_message);
    let mut counter_b: u16 = 0;

    for counter in 0..frame - 1 {
        // println!("Entered the loop");
        let start_index: u16 = (counter * data_size) as u16;
        let end_index: u16 = (start_index + data_size) as u16;

        if counter == frame {
            break;
        }
        let slice: &str = &intermediate_message[start_index as usize..end_index as usize];
        if frame_no == 55 {
            frame_no = 47;
        } else {
            frame_no = frame_no + 1;
        }
        // println!("Printing the sliced message");
        // println!("{}", slice);
        let (sum, cs1, cs2) = checksum(slice, frame_no.into(), ETB);
        println!(" The sum is: {}", sum);
        was_data_sent_successfully = frame_ready(port, slice, frame_no, ETB, cs1, cs2);
        if !was_data_sent_successfully {
            println!("Something wrong with sending data, going back to idle state");
            break;
        } else {
            counter_b = counter + 1;
        }
    }
    if !was_data_sent_successfully {
        return (frame_no, true, was_data_sent_successfully);
    } else {
        // println!("{}",counter_b);
        let start_index: u16 = (counter_b * data_size) as u16;
        let end_index: u16 = (start_index + remainder) as u16;
        let last_frame: &str = &intermediate_message[start_index as usize..end_index as usize];
        // println!("{}", last_frame);
        let (latest_frame, was_data_sent_successfully) =
            send_etx_frame(port, last_frame, frame_no, remainder.into());
        println!(
            "The last frame number in message breakdown function before returning it {}",
            frame_no
        );
        (latest_frame, true, was_data_sent_successfully)
    }
}

/// Here we calculate the checksum. We convert all the characters of the message to their ASCII equivalent decimal values, add them. Then we also add the ASCII equivalent decimal values of the frame number and ETX/ETB. That sum is converted to hex and the LSB's are stored as CS1 and CS2 with CS2 as the LSB
fn checksum(input: &str, frame: u32, etxb: u8) -> (u32, char, char) {
    let mut sum = 0;

    // comment the below 2 lines to calculate the incorrect checksum to test the functionality of sending NAK
    sum += frame;
    sum += etxb as u32;

    for c in input.chars() {
        sum += c as u32;
    }

    // // Discard the most significant byte and store the remaining two bytes
    let hex_sum = sum % 256;
    let cs1: char = format!("{:01X}", (hex_sum & 0b11110000) >> 4)
        .chars()
        .nth(0)
        .unwrap();
    // println!("{}", cs1);

    let cs2: char = format!("{:01X}", hex_sum & 0b00001111)
        .chars()
        .nth(0)
        .unwrap();
    // println!("{}", cs2);
    println!("The sum calculated in checksum is : {}", sum);
    (sum, cs1, cs2)
}

/// Here we match the calculated checksum and the received checksum
fn checksum_match(port: &mut Box<dyn SerialPort>, cs1: char, cs2: char) {
    let mut read_buffer = [0u8];
    match port.read(&mut read_buffer) {
        Ok(_) => {
            if read_buffer[0] == cs1 as u8 {
                println!("CS1 matches");
                port.write(ACK).expect("Write failed");
                match port.read(&mut read_buffer) {
                    Ok(_) => {
                        if read_buffer[0] == cs2 as u8 {
                            println!("CS2 matches");
                            port.write(ACK).expect("Write failed");
                        } else {
                            println!("CS2 does not match, send NAK");
                            port.write_all(NAK).expect("Failed to write [NAK]");
                        }
                    }
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::TimedOut {
                            println!("Timeout occurred. No more data to read.");
                        } else {
                            eprintln!("Error reading from port in checksum CS2 {:?}", e);
                        }
                    }
                }
            } else {
                println!("CS1 does not match, send NAK");
                port.write_all(NAK).expect("Failed to write [NAK]");
            }
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::TimedOut {
                println!("Timeout occurred. No more data to read.");
            } else {
                eprintln!("Error reading from port in checksum CS1 {:?}", e);
            }
        }
    }
}

/// This is the function where we serially send the frame to the receiver
fn frame_ready(
    port: &mut Box<dyn SerialPort>,
    message_to_send: &str,
    frame_no: u8,
    tcc: u8,
    cs1: char,
    cs2: char,
) -> bool {
    let timeout_for_ack = 15;
    let mut retries = 0;
    let mut sent_everything_successfully = false;
    println!("----------------------------Sending new frame---------------------------------");

    while retries < 6 && !sent_everything_successfully {
        println!("Retry count {}", retries);
        // sleep(Duration::from_secs(1));

        let ack_time = Instant::now();

        while Instant::now() - ack_time <= Duration::from_secs(timeout_for_ack) {
            port.write(&[STX]).expect("Write failed");
            port.write(&[frame_no]).expect("Write failed");
            println!("The frame number in frame ready is : {}", frame_no);
            let mut ack_buffer = [0u8; 1]; // u8 tells us the data type of this array/buffer and 0 is the initial value
                                           // let message_to_send_size = size_of_val(message_to_send);
            match port.read(&mut ack_buffer) {
                Ok(_) => {
                    println!("Received byte: {:?}", ack_buffer);
                    if ack_buffer == NAK {
                        println!("Frame numbers dont match");
                        break;
                        // next_frame_setup(port, frame_no, message_to_send, message_to_send_size);
                    } else if ack_buffer == ACK {
                        println!("Frame numbers match");
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Error reading from port in frame ready for frame number matching {:?}",
                        e
                    );
                }
            }
            port.write(message_to_send.as_bytes())
                .expect("Write failed");
            port.write(&[tcc]).expect("Write failed");
            port.write(&[cs1.to_string().as_bytes()[0]])
                .expect("Write failed");
            port.write(&[cs2.to_string().as_bytes()[0]])
                .expect("Write failed");

            let mut checksum_buffer = [0u8; 1]; // u8 tells us the data type of this array/buffer and 0 is the initial value
            match port.read(&mut checksum_buffer) {
                Ok(_) => {
                    if checksum_buffer == NAK {
                        println!("CS1 doesnt match");
                        break;
                        // next_frame_setup(port, frame_no, message_to_send, message_to_send_size);
                    } else if checksum_buffer == ACK {
                        println!("CS1 matches");
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Error reading from port in frame ready for cs1 matching {:?}",
                        e
                    );
                    break;
                }
            }

            let mut checksum_two_buffer = [0u8; 1]; // u8 tells us the data type of this array/buffer and 0 is the initial value
            match port.read(&mut checksum_two_buffer) {
                Ok(_) => {
                    if checksum_two_buffer == NAK {
                        println!("CS2 doesnt match");
                        break;
                        // next_frame_setup(port, frame_no, message_to_send, message_to_send_size);
                    } else if checksum_two_buffer == ACK {
                        println!("CS2 matches");
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Error reading from port in frame ready for cs2 matching {:?}",
                        e
                    );
                    break;
                }
            }

            port.write(&[CR]).expect("Write failed");
            port.write(&[LF]).expect("Write failed");

            if tcc == ETX {
                port.write(&[EOT]).expect("Write failed");
            }

            let mut final_ack_buffer = [0u8; 1];
            match port.read(&mut final_ack_buffer) {
                Ok(_) => {
                    if final_ack_buffer == NAK {
                        println!("Something wrong in the frame");
                        break;
                    } else if final_ack_buffer == ACK {
                        println!("Frame is completely received by the receiver");
                        // return;
                        sent_everything_successfully = true;
                        break;
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Error reading from port in frame ready for ACK for sending frame {:?}",
                        e
                    );
                    break;
                }
            }
        }

        retries += 1;
        // let ack_time = Instant::now();
    }

    if sent_everything_successfully {
        println!("Sent everything from frame_prep() function");
        return sent_everything_successfully;
    } else if retries == 6 {
        println!("Retry count reached 6, going for termination");
        termination_phase(port);
        return sent_everything_successfully;
    } else {
        return sent_everything_successfully;
    }
}

fn termination_phase(port: &mut Box<dyn SerialPort>) {
    println!("Sending EOT to inform end of transmission");
    port.write(&[EOT]).expect("Write failed");
    return;
}

fn reset_states(
    enq_attempts: &mut i32,
    tries_left_for_ack: &mut bool,
    go_for_termination: &mut bool,
    go_for_contention: &mut bool,
    is_connection_established: &mut bool,
    frame_no: &mut u8,
) {
    *enq_attempts = 0;
    *tries_left_for_ack = false;
    *go_for_termination = false;
    *go_for_contention = false;
    *is_connection_established = false;
    *frame_no = 47;
}
