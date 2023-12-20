//! Implements the Programmable Interrupt Controller (PIC) for the x86 architecture.

use bitflags::bitflags;
use x86_64::outb;

/// The base address of the master PIC.
const PIC1: u16 = 0x20;
/// The base address of the slave PIC.
const PIC2: u16 = 0xA0;

/// The command port of the master PIC.
const PIC1_CMD: u16 = PIC1;
/// The data port of the master PIC.
const PIC1_DATA: u16 = PIC1 + 1;
/// The command port of the slave PIC.
const PIC2_CMD: u16 = PIC2;
/// The data port of the slave PIC.
const PIC2_DATA: u16 = PIC2 + 1;

/// The command-code of the end-of-interrupt (EOI) command.
const CMD_CODE_EOI: u8 = 0x20;

/// For initialization, code that indicates that ICW4 will be present.
const CMD_CODE_ICW4: u8 = 0x01;
/// For initialization, code that starts the initialization sequence.
const CMD_CODE_INIT: u8 = 0x10;
/// For initialization, code that indicates that the PIC is in 8086 mode.
const CMD_CODE_ICW4_8086: u8 = 0x01;

/// Initializes the PIC.
pub fn init() {
    unsafe {
        // Start the initialization sequence by sending the initialization command to both PICs.
        outb(PIC1_CMD, CMD_CODE_INIT | CMD_CODE_ICW4);
        wait_a_bit();
        outb(PIC2_CMD, CMD_CODE_INIT | CMD_CODE_ICW4);
        wait_a_bit();

        // Indicate which vector offset the PICs should use.
        outb(PIC1_DATA, super::PIC_OFFSET);
        wait_a_bit();
        outb(PIC2_DATA, super::PIC_OFFSET + 8);
        wait_a_bit();

        // Tell the master PIC that there is a slave PIC at IRQ2 (0000 0100).
        outb(PIC1_DATA, 4);
        wait_a_bit();
        outb(PIC2_DATA, 2);
        wait_a_bit();

        // Use 8086 mode instead of 8080 mode.
        outb(PIC1_DATA, CMD_CODE_ICW4_8086);
        wait_a_bit();
        outb(PIC2_DATA, CMD_CODE_ICW4_8086);
        wait_a_bit();
    }
}

/// Send an END-OF-INTERRUPT command to the PIC for the provided IRQ.
#[inline]
pub fn end_of_interrupt(irq: Irq) {
    if irq as u8 >= 8 {
        unsafe { outb(PIC2_CMD, CMD_CODE_EOI) };
    }

    unsafe { outb(PIC1_CMD, CMD_CODE_EOI) };
}

/// Sets the IRQ mask for the PIC.
#[inline]
pub fn set_irq_mask(masked_irqs: Irqs) {
    unsafe {
        outb(PIC1_DATA, masked_irqs.bits() as u8);
        outb(PIC2_DATA, (masked_irqs.bits() >> 8) as u8);
    }
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
    unsafe { outb(0x80, 0) };
}

/// A possible IRQ number.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[allow(dead_code)]
pub enum Irq {
    /// IRQ 0.
    Zero,
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
    Twelve,
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
        const ZERO = 1 << 0;
        /// IRQ 1.
        const KEYBOARD = 1 << 1;
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
        const TWELVE = 1 << 12;
        /// IRQ 13.
        const THIRTEEN = 1 << 13;
        /// IRQ 14.
        const FOURTEEN = 1 << 14;
        /// IRQ 15.
        const FIFTEEN = 1 << 15;
    }
}
