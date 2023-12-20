//! Access to the Programmable Interval Timer (PIT).

use bitflags::bitflags;
use x86_64::outb;

/// The data port used to send data bytes to the PIT (channel 0).
const PORT_DATA: u16 = 0x40;

/// The command port of the PIT.
const PORT_COMMAND: u16 = 0x43;

bitflags! {
    /// The command codes that can be sent to the PIT.
    struct PitCmd: u8 {
        /// Indicates that the PIT is configured to send a one-time interrupt on IRQ0.
        const CHANNEL_0 = 0b00 << 6;

        /// Data transfered from/to the PIT is read as a sequence of two bytes to make a 16-bit
        /// word.
        ///
        ///
        /// The low byte is sent first, followed by the high byte.
        const ACCESS_MODE_LO_HI = 0b11 << 4;

        /// Indicates that the PIT should send an interrupt at a certain frequency.
        const RATE_GENERATOR = 0b010 << 1;

    }
}

/// Sends a command to the PIT.
#[inline]
fn send_command(cmd: PitCmd) {
    unsafe {
        outb(PORT_COMMAND, cmd.bits());
    }
}

/// Sets the reload value of the PIT, assuming it has been configured
/// with access mode `ACCESS_MODE_LO_HI`.
fn set_reload_value(value: u16) {
    unsafe {
        outb(PORT_DATA, (value & 0xFF) as u8);
        outb(PORT_DATA, ((value >> 8) & 0xFF) as u8);
    }
}

/// Initializes the PIT.
pub fn init() {
    // Send the command to the PIT to configure it to send a one-time interrupt on IRQ0 when the
    // terminal count is reached.
    send_command(PitCmd::CHANNEL_0 | PitCmd::ACCESS_MODE_LO_HI | PitCmd::RATE_GENERATOR);
    set_reload_value(0x1234);
}
