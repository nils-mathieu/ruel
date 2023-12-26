//! PCI bus driver.

use core::mem::{transmute, MaybeUninit};

use ruel_sys::PciDevice;
use x86_64::{inl, outl};

use crate::global::OutOfMemory;
use crate::log;
use crate::utility::BumpAllocator;

/// Computes the address within the PCI configuration space.
///
/// # Panics
///
/// This function panics if any of the provided parameters are out of bound
/// or mis-aligned.
fn pci_address(bus: u32, device: u32, func: u32) -> u32 {
    debug_assert!(bus < 256);
    debug_assert!(device < 32);
    debug_assert!(func < 8);

    0x8000_0000 | (bus << 16) | (device << 11) | (func << 8)
}

/// Reads a word from the configuration space of the PCI device at the provided coordinates
/// (bus, device, function, offset).
fn config_read_u32(address: u32) -> u32 {
    unsafe {
        outl(0xCF8, address);

        // The offset gave us two values, we need to shift the result to the right
        // position (based on the offset).
        u32::from_le(inl(0xCFC))
    }
}

/// Returns whether the PCI device at the provided coordinates is present.
fn is_present(bus: u32, device: u32, func: u32) -> bool {
    let off0 = config_read_u32(pci_address(bus, device, func));
    let vendor_id = off0 as u16;

    // The vendor ID 0xFFFF is not valid, so it is used to indicate that the device is not
    // present.
    vendor_id != 0xFFFF
}

/// Returns whether the provided PCI device has multiple functions.
fn has_multiple_functions(bus: u32, device: u32) -> bool {
    let off12 = config_read_u32(pci_address(bus, device, 0) + 12);
    let header_type = (off12 >> 16) as u8;

    // Bit 7 indicates whether the device is a multi-function device.
    header_type & 0x80 != 0
}

/// Contains the common header of a PCI device.
#[derive(Debug, Clone, Copy)]
#[repr(C, align(4))]
struct CommonHeader {
    /// The vendor ID of the device.
    pub vendor_id: u16,
    /// The ID of the device.
    pub device_id: u16,
    /// The command register.
    ///
    /// This is used to control the device's ability to generate and respond to PCI cycles. When
    /// zero is written to this register, the device is disabled from responding to PCI cycles.
    pub command: u16,
    /// The status register.
    ///
    /// This contains status information for PCI bus related events.
    pub status: u16,
    /// The revision ID of the device.
    pub revision_id: u8,
    /// The programming interface.
    ///
    /// This is the register-level programming interface the device has, if any.
    pub prog_if: u8,
    /// The subclass code.
    ///
    /// Specifies the specific function of the device.
    pub subclass: u8,
    /// The class code.
    ///
    /// The class code is used to identify the generic function of the device.
    pub class_code: u8,
    /// The cache line size of the device, as a multiple of a 32-bit cache line.
    ///
    /// If an invalid value is written to this register, it will behave as if the value 0
    /// had been written.
    pub cache_line_size: u8,
    /// The latency timer of the device, as a multiple of the PCI bus clock period.
    pub latency_timer: u8,
    /// The header type of the device.
    ///
    /// * 0x00 - Standard header
    /// * 0x01 - PCI-to-PCI bridge
    /// * 0x02 - CardBus bridge
    ///
    /// Bit 7 indicates whether the device is a multi-function device.
    pub header_type: u8,
    /// Allows control of the device's Built-In Self Test (BIST) capability.
    pub bist: u8,
}

impl CommonHeader {
    /// Reads the common header of the PCI device at the provided coordinates.
    ///
    /// # Errors
    ///
    /// This function returns [`None`] if the device is not present on the bus.
    pub fn read(address: u32) -> Self {
        let off0 = config_read_u32(address);
        let vendor_id = off0 as u16;
        let device_id = (off0 >> 16) as u16;

        let off4 = config_read_u32(address + 4);
        let command = off4 as u16;
        let status = (off4 >> 16) as u16;

        let off8 = config_read_u32(address + 8);
        let revision_id = off8 as u8;
        let prog_if = (off8 >> 8) as u8;
        let subclass = (off8 >> 16) as u8;
        let class_code = (off8 >> 24) as u8;

        let off12 = config_read_u32(address + 12);
        let cache_line_size = off12 as u8;
        let latency_timer = (off12 >> 8) as u8;
        let header_type = (off12 >> 16) as u8;
        let bist = (off12 >> 24) as u8;

        Self {
            device_id,
            vendor_id,
            status,
            command,
            class_code,
            subclass,
            prog_if,
            revision_id,
            bist,
            header_type,
            latency_timer,
            cache_line_size,
        }
    }
}

/// Calls the provided function for every detected PCI device.
///
/// The parameter of the function is the device's address in the configuration
/// space of the PCI bus.
fn for_each_device(mut f: impl FnMut(u32)) {
    for bus in 0..256 {
        for device in 0..32 {
            if !is_present(bus, device, 0) {
                continue;
            }

            f(pci_address(bus, device, 0));

            if has_multiple_functions(bus, device) {
                for func in 1..8 {
                    if is_present(bus, device, func) {
                        f(pci_address(bus, device, func))
                    }
                }
            }
        }
    }
}

/// Counts the number of available PCI devices.
fn count_pci_devices() -> usize {
    let mut count = 0;
    for_each_device(|_| count += 1);
    count
}

/// Initializes the PCI bus driver.
pub fn init(
    bootstrap_allocator: &mut BumpAllocator,
) -> Result<&'static mut [PciDevice], OutOfMemory> {
    // Read the header type of the null device (bus 0, device 0, function 0).
    log::trace!("Enumerating PCI devices...");
    let count = count_pci_devices();

    let devices = bootstrap_allocator.allocate_slice::<PciDevice>(count)?;

    let mut index = 0;
    for_each_device(|address| {
        let common_header = CommonHeader::read(address);
        debug_assert!(common_header.vendor_id != 0xFFFF);

        devices[index].write(PciDevice {
            address,
            id: common_header.device_id,
            vendor: common_header.vendor_id,
        });
        index += 1;
    });

    log::trace!("Found {count} PCI devices!");
    unsafe { Ok(transmute::<&mut [MaybeUninit<PciDevice>], &mut [PciDevice]>(devices)) }
}
