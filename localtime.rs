// Thin Rust FFI layer around libtz
//
// Copyright © 2023 David Caldwell <david@porkrind.org>
// License: MIT (see LICENSE.md file)

#![doc = include_str!("README.md")]

use std::cell::RefCell;
use std::ffi::{CString,CStr};
use std::os::unix::ffi::OsStringExt;
use std::os::raw::{c_void,c_char,c_int,c_long};

thread_local!(static ENV_STORAGE: RefCell<CString> = RefCell::new(CString::default()));

#[doc(hidden)]
#[no_mangle]
pub extern "C" fn rust_getenv(name: *const c_char) -> *const c_char {
    rust_getenv_internal(name).unwrap_or(std::ptr::null())
}

fn rust_getenv_internal(name: *const c_char) -> Option<*const c_char> {
    let name: &str = unsafe { CStr::from_ptr(name).to_str().ok()? };
    let value_cstr = CString::new(std::env::var_os(name)?.into_vec()).ok()?;
    ENV_STORAGE.with(|storage_cell| {
        let mut storage = storage_cell.borrow_mut();
        *storage = value_cstr;
        Some(storage.as_ptr())
    })
}

/// System time. On unix, the number of seconds since 00:00:00 UTC on 1 January 1970.
///
/// Note: This is libtz's `time_t`, which currently hardcoded to i64 regardless
/// of the system's `time_t`.
pub type TimeT = i64; // See `-Dtime_tz` in build.rs

/// A broken down time representation, logically equivalent to `struct tm` in unix.
///
/// Note: This is libtz's `struct tm` and doesn't necessarily match the system's
/// in terms of member ordering.
///
/// Reference: <https://pubs.opengroup.org/onlinepubs/7908799/xsh/time.h.html>
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Tm {
    /** Seconds          [0, 60] */                 pub tm_sec   : c_int,
    /** Minutes          [0, 59] */                 pub tm_min   : c_int,
    /** Hour             [0, 23] */                 pub tm_hour  : c_int,
    /** Day of the month [1, 31] */                 pub tm_mday  : c_int,
    /** Month            [0, 11]  (January = 0) */  pub tm_mon   : c_int,
    /** Year minus 1900 */                          pub tm_year  : c_int,
    /** Day of the week  [0, 6]   (Sunday = 0) */   pub tm_wday  : c_int,
    /** Day of the year  [0, 365] (Jan/01 = 0) */   pub tm_yday  : c_int,
    /** Daylight savings flag */                    pub tm_isdst : c_int,

    /** Seconds East of UTC */                      pub tm_gmtoff : c_long,
    /** Timezone abbreviation */                    pub tm_zone: *const c_char,
}

/// Opaque pointer for timezone data. Defined in the C code as:
/// ```C
/// typedef struct state *timezone_t;
/// ```
///
/// This is returned by [`tzalloc`]. It should be explicitly freed with [`tzfree`].
pub type TimezoneT = *const c_void;

extern "C" {
    /// Convert system time to UTC.
    ///
    /// Rust interface for the C function:
    /// ```C
    /// struct tm *gmtime_r(time_t const *restrict, struct tm *restrict);
    /// ```
    ///
    /// The `gmtime_r` function converts to Coordinated Universal Time, returning a pointer to [`Tm`]
    /// structures, described below. The storage for the returned [`Tm`] should be passed in as the 2nd
    /// argument. If it encounters an error, it will return a `NULL` and set `errno` (see
    /// [`std::io::Error::last_os_error`]).
    #[link_name = "tz_gmtime_r"]
    pub fn gmtime_r(timep: *const TimeT, tmp: *mut Tm) -> *mut Tm;

    /// Convert system time to local time using globally configured time zone.
    ///
    /// Rust interface for the C function:
    /// ```C
    /// struct tm *localtime_r(time_t const *restrict, struct tm *restrict);
    /// ```
    ///
    /// The `localtime_r` function corrects for the time zone and any time zone adjustments (such as Daylight
    /// Saving Time in the United States).  The storage for the returned [`Tm`] should be passed in as the 2nd
    /// argument. If it encounters an error, it will return a `NULL` and set `errno` (see
    /// [`std::io::Error::last_os_error`]). The pointer returned in [`Tm::tm_zone`] will only be valid until
    /// the next [`tzet()`][tzset] call.
    #[link_name = "tz_localtime_r"]
    pub fn localtime_r(timep: *const TimeT, tmp: *mut Tm) -> *mut Tm;

    /// Convert UTC `Tm` to system time.
    ///
    /// Rust interface for the C function:
    /// ```C
    /// time_t timegm(struct tm *);
    /// ```
    ///
    /// This function is like [`mktime`] except that it treats the `tmp` as UTC (ignoring the `tm_idst` and
    /// `tm_zone` members).
    #[link_name = "tz_timegm"]
    pub fn timegm(tmp: *const Tm) -> TimeT;

    /// Convert local time `Tm` to system time using globally configured time zone.
    ///
    /// Rust interface for the C function:
    /// ```C
    /// time_t mktime(struct tm *);
    /// ```
    ///
    /// The `mktime` function converts the broken-down time, expressed as local time, in the structure pointed
    /// to by `tm` into a calendar time value with the same encoding as that of the values returned by the
    /// `time` function.  The original values of the `tm_wday` and `tm_yday` components of the structure are
    /// ignored, and the original values of the other components are not restricted to their normal ranges.
    /// (A positive or zero value for `tm_isdst` causes `mktime` to presume initially that daylight saving
    /// time respectively, is or is not in effect for the specified time.
    ///
    /// A negative value for `tm_isdst` causes the `mktime` function to attempt to divine whether daylight
    /// saving time is in effect for the specified time; in this case it does not use a consistent rule and
    /// may give a different answer when later presented with the same argument.)  On successful completion,
    /// the values of the `tm_wday` and `tm_yday` components of the structure are set appropriately, and the
    /// other components are set to represent the specified calendar time, but with their values forced to
    /// their normal ranges; the final value of `tm_mday` is not set until `tm_mon` and `tm_year` are
    /// determined.  The `mktime` function returns the specified calendar time; If the calendar time cannot be
    /// represented, it returns -1.
    #[link_name = "tz_mktime"]
    pub fn mktime(tmp: *const Tm) -> TimeT;

    /// Re-read `TZ` environment variable and configure global time zone.
    ///
    /// Rust interface for the C function:
    /// ```C
    /// void tzset(void);
    /// ```
    ///
    /// The `tzset` function acts like `tzalloc(getenv("TZ"))`, except it saves any resulting timezone object
    /// into internal storage that is accessed by `localtime_r`, and `mktime`.  The anonymous shared timezone
    /// object is freed by the next call to `tzset`.  If the implied call to `tzalloc` fails, `tzset` falls
    /// back on Universal Time (UT).
    #[link_name = "tz_tzset"]
    pub fn tzset();

    // -DNETBSD_INSPIRED
    /// Allocate a configured time zone.
    ///
    /// Rust interface for the C function:
    /// ```C
    /// timezone_t tzalloc(char const *);
    /// ```
    ///
    /// The `tzalloc` function allocates and returns a timezone object described
    /// by `TZ`.  If `TZ` is not a valid timezone description,  or  if  the  object
    /// cannot be allocated, `tzalloc` returns a null pointer and sets `errno`.
    ///
    #[doc = include_str!("tzalloc.md")]
    #[link_name = "tz_tzalloc"]
    pub fn tzalloc(zone: *const c_char) -> TimezoneT;

    /// Free allocated time zone.
    ///
    /// Rust interface for the C function:
    /// ```C
    /// void tzfree(timezone_t);
    /// ```
    ///
    /// The `tzfree` function frees a timezone object `tz`, which should have been successfully allocated by
    /// `tzalloc`.  This invalidates any `tm_zone` pointers that `tz` was used to set.
    #[link_name = "tz_tzfree"]
    pub fn tzfree(tz: TimezoneT);

    /// Convert system time to local time using passed-in time zone.
    ///
    /// Rust interface for the C function:
    /// ```C
    /// struct tm *localtime_rz(timezone_t restrict, time_t const *restrict, struct tm *restrict);
    /// ```
    ///
    /// This acts like [`localtime_r`] except it uses the passed in TimezoneT instead of the shared global
    /// configuration. The pointer returned in [`Tm::tm_zone`] will be valid until the [`TimezoneT`] is freed
    /// with [`tzfree()`][tzfree].
    #[link_name = "tz_localtime_rz"]
    pub fn localtime_rz(tz: TimezoneT, timep: *const TimeT, tmp: *mut Tm) -> *mut Tm;

    /// Convert local time `Tm` to system time using passed-in time zone.
    ///
    /// Rust interface for the C function:
    /// ```C
    /// time_t mktime_z(timezone_t restrict, struct tm *restrict);
    /// ```
    ///
    /// This acts like [`mktime`] except it uses the passed in TimezoneT instead of the shared global
    /// configuration.
    #[link_name = "tz_mktime_z"]
    pub fn mktime_z(tz: TimezoneT, tmp: *const Tm) -> TimeT;

    /// Convert from leap-second to POSIX `time_t`s.
    ///
    /// Rust interface for the C function:
    /// ```C
    /// time_t posix2time_z(timezone_t, time_t);
    /// ```
    ///
    #[doc = include_str!("time2posix.md")]
    #[link_name = "tz_posix2time_z"]
    pub fn posix2time_z(tz: TimezoneT, t: TimeT) -> TimeT;

    /// Convert from POSIX to leap-second `time_t`s.
    ///
    /// Rust interface for the C function:
    /// ```C
    /// time_t time2posix_z(timezone_t, time_t);
    /// ```
    ///
    #[doc = include_str!("time2posix.md")]
    #[link_name = "tz_time2posix_z"]
    pub fn time2posix_z(tz: TimezoneT, t: TimeT) -> TimeT;
}

#[cfg(test)]
mod tests {
    use std::ffi::{CString,CStr};
    use std::mem::MaybeUninit;
    use super::*;
    #[test]
    fn basic() {
        let time: TimeT = 127810800;
        std::env::set_var("TZ", "America/Los_Angeles");
        unsafe { tzset() };
        let mut tm = MaybeUninit::<Tm>::uninit();
        let tmp = tm.as_mut_ptr();
        let ret = unsafe { localtime_r(&time, tmp) };
        assert_ne!(ret, std::ptr::null_mut());
        assert_eq!(ret, tmp);
        let tm = unsafe { tm.assume_init() };
        let zone: &str = unsafe { CStr::from_ptr(tm.tm_zone).to_str().expect("correct utf8") };
        assert_eq!((tm.tm_sec, tm.tm_min, tm.tm_hour, tm.tm_mday, tm.tm_mon, tm.tm_year, tm.tm_wday, tm.tm_yday, tm.tm_isdst, tm.tm_gmtoff, zone),
                   (0,         0,         0,          19,         0,         74,         6,          18,         1,           -25200,       "PDT"));
        assert_eq!(unsafe { mktime(&tm) }, time); // Round trip
        let time: TimeT = time + tm.tm_gmtoff;
        assert_eq!(unsafe { timegm(&tm) }, time);

        let mut tm = MaybeUninit::<Tm>::uninit();
        let tmp = tm.as_mut_ptr();
        let ret = unsafe { gmtime_r(&time, tmp) };
        assert_ne!(ret, std::ptr::null_mut());
        assert_eq!(ret, tmp);
        let tm = unsafe { tm.assume_init() };
        let zone: &str = unsafe { CStr::from_ptr(tm.tm_zone).to_str().expect("correct utf8") };
        assert_eq!((tm.tm_sec, tm.tm_min, tm.tm_hour, tm.tm_mday, tm.tm_mon, tm.tm_year, tm.tm_wday, tm.tm_yday, tm.tm_isdst, tm.tm_gmtoff, zone),
                   (0,         0,         0,          19,         0,         74,         6,          18,         0,           0,           "UTC"));
    }

    #[test]
    fn localtime_rz_test() {
        let tzname = CString::new("America/New_York").unwrap();
        let tz = unsafe { tzalloc(tzname.as_ptr()) };
        assert_ne!(tz, std::ptr::null_mut());
        let time: TimeT = 127810800;
        let mut tm = MaybeUninit::<Tm>::uninit();
        let ret = unsafe { localtime_rz(tz, &time, tm.as_mut_ptr()) };
        assert_ne!(ret, std::ptr::null_mut());
        let tm = unsafe { tm.assume_init() };
        let zone: &str = unsafe { CStr::from_ptr(tm.tm_zone).to_str().expect("correct utf8") };
        assert_eq!((tm.tm_sec, tm.tm_min, tm.tm_hour, tm.tm_mday, tm.tm_mon, tm.tm_year, tm.tm_wday, tm.tm_yday, tm.tm_isdst, tm.tm_gmtoff, zone),
                   (0,         0,         3,          19,         0,         74,         6,          18,         1,           -14400,       "EDT"));
        assert_eq!(unsafe { mktime_z(tz, &tm) }, time); // Round trip
        unsafe { tzfree(tz) };
    }

    #[test]
    fn posix_conversions() {
        let posixtime: TimeT = 536457599;
        let tzname = CString::new("UTC").unwrap();
        let tz = unsafe { tzalloc(tzname.as_ptr()) };
        assert_ne!(tz, std::ptr::null_mut());
        let time = unsafe { posix2time_z(tz, posixtime) };
        let mut tm = MaybeUninit::<Tm>::uninit();
        let ret = unsafe { localtime_rz(tz, &time, tm.as_mut_ptr()) };
        assert_ne!(ret, std::ptr::null_mut());
        let tm = unsafe { tm.assume_init() };
        let zone: &str = unsafe { CStr::from_ptr(tm.tm_zone).to_str().expect("correct utf8") };
        assert_eq!((tm.tm_sec, tm.tm_min, tm.tm_hour, tm.tm_mday, tm.tm_mon, tm.tm_year, tm.tm_wday, tm.tm_yday, tm.tm_isdst, tm.tm_gmtoff, zone),
                   (59,        59,        23,         31,         11,        86,         3,          364,        0,           0,            "UTC"));
        assert_eq!(unsafe { time2posix_z(tz, time) }, posixtime); // Round Trip
        unsafe { tzfree(tz) };
    }

    #[test]
    fn test_readme_deps() {
        version_sync::assert_markdown_deps_updated!("README.md");
    }
}
