libtz-sys
=========

Rust FFI interface for IANA's [libtz](https://www.iana.org/time-zones)
([git repository](https://github.com/eggert/tz)).

This is a low level library---You will _most likely_ prefer [libtz, a more
idomatic Rust interface built on top of this](https://github.com/caldwell/libtz).

This provides an equivalent of glibc's `localtime_r()` function (and related
functions). The difference is that this library has been compiled such that the
`getenv("TZ")` call uses Rust's `std::env::var_os()` which protects it from
`std::env::set_var()` races which could otherwise cause segfaults.

Aside from that it should be a drop in replacement for glibc. It will read the
tzdata files that the system has installed to calculate things like leap seconds
and daylight saving time.

Links: [[Documentation](https://docs.rs/libtz-sys/latest)]
       [[Git Repository](https://github.com/caldwell/libtz-sys)]
       [[Crates.io](https://crates.io/crates/libtz-sys)]

Usage
-----

Add this to your `Cargo.toml`:

```toml
[dependencies]
libtz-sys = "0.2.1"
```

Example
-------

```rust
use std::ffi::{CString,CStr};
use std::mem::MaybeUninit;
use libtz_sys::{tzalloc, localtime_rz, mktime_z, tzfree, TimeT, Tm};

let tzname = CString::new("America/New_York").unwrap();
let tz = unsafe { tzalloc(tzname.as_ptr()) };
if tz == std::ptr::null_mut() {
    return Err(std::io::Error::last_os_error());
}
let time: TimeT = 127810800;
let mut tm = MaybeUninit::<Tm>::uninit();
let ret = unsafe { localtime_rz(tz, &time, tm.as_mut_ptr()) };
if ret == std::ptr::null_mut() {
    return Err(std::io::Error::last_os_error());
}
let tm = unsafe { tm.assume_init() };
let zone: &str = unsafe { CStr::from_ptr(tm.tm_zone).to_str().expect("correct utf8") };
assert_eq!((tm.tm_sec, tm.tm_min, tm.tm_hour, tm.tm_mday, tm.tm_mon),
           (0,         0,         3,          19,         0,       ));
assert_eq!((tm.tm_year, tm.tm_wday, tm.tm_yday, tm.tm_isdst, tm.tm_gmtoff, zone),
           (74,         6,          18,         1,           -14400,       "EDT"));

let time_again = unsafe { mktime_z(tz, &tm) }; // Round trip
if time_again == -1 {
    // Didn't work (errno is not reliably set in this case)
} else {
    assert_eq!(time_again, time);
}
unsafe { tzfree(tz) };
# Ok(())
```

License
-------

The Rust code is distributed under the MIT license.

The libtz code is mostly public domain with a couple files using the BSD-3
clause license.

See [LICENSE.md](LICENSE.md) for details.
