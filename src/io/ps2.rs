//! An implementation of the PS/2 controller.
//!
//! The main documentation for writing this module was taken from the OSDev Wiki (as always, they
//! are awesome). The page can be found [here][wiki].
//!
//! [wiki]: (https://wiki.osdev.org/%228042%22_PS/2_Controller#Translation)

use bitflags::bitflags;
use x86_64::{inb, outb};

use crate::log;

/// An error that might occur while communicating with the PS/2 controller.
#[derive(Clone, Copy, Debug)]
pub enum PS2Error {
    /// The PS/2 controller did not respond in time.
    Timeout,
}

impl core::fmt::Display for PS2Error {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            PS2Error::Timeout => write!(f, "PS/2 controlled timed out"),
        }
    }
}

/// Reads the status register of the PS/2 controller.
#[inline]
pub fn status() -> PS2Status {
    let raw = unsafe { inb(0x64) };
    PS2Status::from_bits_retain(raw)
}

/// Sends a command to the PS/2 controller.
#[inline]
pub fn command(cmd: u8) {
    unsafe { outb(0x64, cmd) }
}

/// Reads the data register of the PS/2 controller.
///
/// # Remarks
///
/// This function does not check whether the output buffer actually contain any meaningful data.
/// The caller is responsible for checking the status register before calling this function, or
/// having another means of knowing whether the output buffer contains meaningful data, such as
/// having received an interrupt from the PS/2 controller.
#[inline]
pub fn read_data() -> u8 {
    unsafe { inb(0x60) }
}

/// Sends data to the PS/2 controller.
#[inline]
pub fn write_data(data: u8) {
    unsafe { outb(0x60, data) }
}

// /// Writes to the data register of the PS/2 controller.
// #[inline]
// pub fn write_data(data: u8) {
//     unsafe { outb(0x60, data) }
// }

bitflags! {
    /// Represents the status register of the PS/2 controller.
    #[derive(Clone, Copy, Debug)]
    pub struct PS2Status: u8 {
        /// Indicates that the output buffer of the controller is full.
        ///
        /// This bit must be set when attempting to read the data register.
        const OUTPUT_BUFFER_FULL = 1 << 0;

        /// Indicates that the input buffer of the controller is full.
        ///
        /// This bit must be clear when attempting to write to the data or command register.
        const INPUT_BUFFER_FULL = 1 << 1;

        /// This bit is meant to be cleared when the controller is reset. It is then set again
        /// by the firmware.
        const SYSTEM = 1 << 2;

        /// When this bit is set, the data written to the input buffer is meant for the PS/2
        /// controller command.
        ///
        /// When this bit is clear, the data written to the input buffer is meant for the PS/2
        /// device.
        const COMMAND = 1 << 3;


        /// Indicates that the data present in the output buffer is from the second PS/2 port.
        ///
        /// # Remarks
        ///
        /// This is apparently not reliable on some hardware, as the meaning of this bit
        /// is not well-defined.
        ///
        /// Works well enough :p
        const AUX_OUTPUT_BUFFER_FULL = 1 << 5;

        /// Indicates that a timeout error has occurred.
        const TIMEOUT_ERROR = 1 << 6;
        /// Indicates that a parity error has occurred.
        const PARITY_ERROR = 1 << 7;
    }
}

const WAIT_ITERATIONS: usize = 10000;

/// Waits for the output buffer of the PS/2 controller to be full.
fn wait_output_buffer_full() -> Result<(), PS2Error> {
    for _ in 0..WAIT_ITERATIONS {
        if status().intersects(PS2Status::OUTPUT_BUFFER_FULL) {
            return Ok(());
        }

        core::hint::spin_loop();
    }

    Err(PS2Error::Timeout)
}

/// Waits for the input buffer of the PS/2 controller to be empty.
#[must_use = "thisfunction might've failed"]
fn wait_input_buffer_empty() -> Result<(), PS2Error> {
    for _ in 0..WAIT_ITERATIONS {
        if !status().intersects(PS2Status::INPUT_BUFFER_FULL) {
            return Ok(());
        }

        core::hint::spin_loop();
    }

    Err(PS2Error::Timeout)
}

/// Writes a byte to the auxiliary device.
fn aux_write_data(val: u8) -> Result<(), PS2Error> {
    wait_input_buffer_empty()?;
    command(0xD4); // Indicates that the next byte is meant for the auxilliary device
    wait_input_buffer_empty()?;
    write_data(val);
    Ok(())
}

/// Reads a byte from the auxiliary device.
fn aux_read_data() -> Result<u8, PS2Error> {
    wait_output_buffer_full()?;
    Ok(read_data())
}

/// Initializes the auxilliary PS/2 controller (mouse).
fn init_aux_device() -> Result<(), PS2Error> {
    // Enable the auxilliary device.
    wait_input_buffer_empty()?;
    command(0xA8);

    // Enable receiving interrupts from the auxilliary device.
    wait_input_buffer_empty()?;
    command(0x20);
    wait_output_buffer_full()?;
    let current_status = read_data();
    wait_input_buffer_empty()?;
    command(0x60);
    wait_input_buffer_empty()?;
    write_data(current_status | (1 << 1));

    // Tell the mouse to use default settings.
    aux_write_data(0xF6)?;
    aux_read_data()?; // ack

    // Enable the mouse.
    aux_write_data(0xF4)?;
    aux_read_data()?; // ack

    Ok(())
}

/// Initializes the PS/2 controller.
pub fn init() -> Result<(), PS2Error> {
    log::trace!("Initializing the PS/2 controller...");
    init_aux_device()?;
    Ok(())
}
