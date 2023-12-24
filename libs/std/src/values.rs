use core::time::Duration;

/// Returns the number of ticks since the system was booted.
///
/// See [`Value::UPTICKS`] for more information.
pub fn upticks() -> u64 {
    let mut result = 0;
    let _ret = sys::read_value(sys::Value::UPTICKS, &mut result as *mut _ as *mut u8);
    debug_assert_eq!(_ret, sys::SysResult::SUCCESS);
    result
}

/// Returns the number of nanoseconds that each tick spans for.
///
/// See [`Value::NANOSECONDS_PER_TICK`] for more information.
pub fn tick_duration_ns() -> u32 {
    let mut result = 0;
    let _ret = sys::read_value(
        sys::Value::NANOSECONDS_PER_TICK,
        &mut result as *mut _ as *mut u8,
    );
    debug_assert_eq!(_ret, sys::SysResult::SUCCESS);
    result
}

/// Returns the duration of a single CPU tick.
///
/// See [`Value::NANOSECONDS_PER_TICK`] for more information.
#[inline]
pub fn tick_duration() -> Duration {
    Duration::from_nanos(tick_duration_ns() as u64)
}

/// Returns the amount of time that elapsed since the system was booted.
pub fn uptime() -> Duration {
    let mut result = sys::Duration::ZERO;
    let _ret = sys::read_value(sys::Value::UPTIME, &mut result as *mut _ as *mut u8);
    debug_assert_eq!(_ret, sys::SysResult::SUCCESS);
    Duration::new(result.seconds, result.nanoseconds as u32)
}
