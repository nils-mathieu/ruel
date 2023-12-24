//! Implements the Programmable Interrupt Controller (PIC) for the x86 architecture.

use bitflags::bitflags;
use x86_64::outb;

/// A PIC (Programmable Interrupt Controller).
struct Pic {
    cmd: u16,
    data: u16,
}

impl Pic {
    /// The first PIC.
    pub const MASTER: Self = Self {
        cmd: 0x20,
        data: 0x21,
    };

    /// The second PIC.
    pub const SLAVE: Self = Self {
        cmd: 0xA0,
        data: 0xA1,
    };

    /// Sends a command byte to the PIC.
    #[inline]
    pub fn command(self, cmd: u8) {
        unsafe { outb(self.cmd, cmd) }
    }

    /// Writes data to the PIC.
    #[inline]
    pub fn write(self, data: u8) {
        unsafe { outb(self.data, data) }
    }
}

/// Initializes the PIC.
pub fn init() {
    // ICW stands for "Initialization Command Word" btw.

    // Start the initialization sequence by sending the initialization command to both PICs.
    //
    // bit 0 - indicates that ICW4 is needed.
    // bit 1 - cascade mode (we're using a master/slave configuration).
    // bit 2 - call address interval (interval of 8)
    // bit 3 - edge triggered mode
    // bit 4 - start initialization sequence (this bit is required to start the initialization
    //         sequence).
    Pic::MASTER.command(0x11);
    wait_a_bit();
    Pic::SLAVE.command(0x11);
    wait_a_bit();

    // Indicate which vector offset the PICs should use.
    //
    // This is ICW2.
    Pic::MASTER.write(super::PIC_OFFSET);
    wait_a_bit();
    Pic::SLAVE.write(super::PIC_OFFSET + 8);
    wait_a_bit();

    // Tell the master PIC that there is a slave PIC at IRQ2 (0000 0100).
    //
    // This is ICW3 (master and slave don't have the same meaning at that point).
    Pic::MASTER.write(1 << 2);
    wait_a_bit();
    Pic::SLAVE.write(1 << 1);
    wait_a_bit();

    // Use 8086 mode instead of 8085 mode.
    //
    // This is ICW4 (we requested it in the first command).
    Pic::MASTER.write(0x01);
    wait_a_bit();
    Pic::SLAVE.write(0x01);
    wait_a_bit();
}

/// Send an END-OF-INTERRUPT command to the PIC for the provided IRQ.
#[inline]
pub fn end_of_interrupt(irq: Irq) {
    // EOI is bit 5 of the operation command word (OCW2).
    // That word is sent to the command register.

    if irq as u8 >= 8 {
        Pic::SLAVE.command(1 << 5);
    }

    Pic::MASTER.command(1 << 5);
}

/// Sets the IRQ mask for the PIC.
#[inline]
pub fn set_irq_mask(masked_irqs: Irqs) {
    // OCW1 is the operation command word 1. It contains a mask
    // of the IRQs that should be disabled.
    // That word is sent to the data register.

    Pic::MASTER.write(masked_irqs.bits() as u8);
    Pic::SLAVE.write((masked_irqs.bits() >> 8) as u8);
}

/// Perform an operation that takes a bit of time to complete but has no side effects. This is
/// needed because some older machines are too fast for the PIC to keep up with, so we need to
/// wait a bit after sending a command to the PIC.
///
/// This function takes between 1 to 4 microseconds to complete.
#[inline]
fn wait_a_bit() {
    // Any unused port works for this. Linux uses 0x80, so we'll use that too. It's almost always
    // unused after boot.
    unsafe { outb(0x80, 0u8) };
}

/// A possible IRQ number.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[allow(dead_code)]
pub enum Irq {
    /// IRQ 0.
    Timer,
    /// IRQ 1.
    PS2Keyboard,
    /// IRQ 2.
    Two,
    /// IRQ 3.
    Three,
    /// IRQ 4.
    Four,
    /// IRQ 5.
    Five,
    /// IRQ 6.
    Six,
    /// IRQ 7.
    Seven,
    /// IRQ 8.
    Eight,
    /// IRQ 9.
    Nine,
    /// IRQ 10.
    Ten,
    /// IRQ 11.
    Eleven,
    /// IRQ 12.
    PS2Mouse,
    /// IRQ 13.
    Thirteen,
    /// IRQ 14.
    Fourteen,
    /// IRQ 15.
    Fifteen,
}

bitflags! {
    /// A set of IRQs.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Irqs: u16 {
        /// IRQ 0.
        const TIMER = 1 << 0;
        /// IRQ 1.
        const PS2_KEYBOARD = 1 << 1;
        /// IRQ 2.
        const TWO = 1 << 2;
        /// IRQ 3.
        const THREE = 1 << 3;
        /// IRQ 4.
        const FOUR = 1 << 4;
        /// IRQ 5.
        const FIVE = 1 << 5;
        /// IRQ 6.
        const SIX = 1 << 6;
        /// IRQ 7.
        const SEVEN = 1 << 7;
        /// IRQ 8.
        const EIGHT = 1 << 8;
        /// IRQ 9.
        const NINE = 1 << 9;
        /// IRQ 10.
        const TEN = 1 << 10;
        /// IRQ 11.
        const ELEVEN = 1 << 11;
        /// IRQ 12.
        const PS2_MOUSE = 1 << 12;
        /// IRQ 13.
        const THIRTEEN = 1 << 13;
        /// IRQ 14.
        const FOURTEEN = 1 << 14;
        /// IRQ 15.
        const FIFTEEN = 1 << 15;
    }
}
