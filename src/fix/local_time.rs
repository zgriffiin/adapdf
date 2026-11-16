//! The local time on Windows, for the name of the backup. The one call here
//! is the only `unsafe` code outside `src/word`.

use windows::Win32::System::SystemInformation::GetLocalTime;

use super::Timestamp;

/// The current local time, from the system clock and time zone.
pub fn now() -> Timestamp {
    // SAFETY: `GetLocalTime` has no arguments that can be wrong. It returns the
    // time by value and cannot fail.
    let time = unsafe { GetLocalTime() };
    Timestamp {
        year: i64::from(time.wYear),
        month: u32::from(time.wMonth),
        day: u32::from(time.wDay),
        hour: u32::from(time.wHour),
        minute: u32::from(time.wMinute),
        second: u32::from(time.wSecond),
    }
}
