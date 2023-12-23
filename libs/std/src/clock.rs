use core::time::Duration;

/// Returns the number of nanoseconds since the system was booted.
pub fn raw_uptime() -> u64 {
    let mut result = 0;
    let _ret = sys::read_clock(sys::ClockId::UPTIME, &mut result as *mut _ as *mut u8);
    debug_assert_eq!(_ret, sys::SysResult::SUCCESS);
    result
}

/// Returns the amount of time that the system has been running for.
#[inline]
pub fn uptime() -> Duration {
    Duration::from_nanos(raw_uptime())
}

/// Returns the number of ticks since the system was booted.
pub fn upticks() -> u64 {
    let mut result = 0;
    let _ret = sys::read_clock(sys::ClockId::UPTICKS, &mut result as *mut _ as *mut u8);
    debug_assert_eq!(_ret, sys::SysResult::SUCCESS);
    result
}
