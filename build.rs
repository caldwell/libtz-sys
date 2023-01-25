// Copyright © 2023 David Caldwell <david@porkrind.org>

extern crate make_cmd;
use make_cmd::make;

fn main() {
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("no OUT_DIR from cargo"));

    for file in ["tz/localtime.c", "tz/asctime.c", "tz/difftime.c", "tz/strftime.c", "tz/tzfile.h", "tz/private.h"].into_iter() {
        println!("cargo:rerun-if-changed={}", file);
    }

    let mut make = make();
    make.arg("libtz.a")
        .arg(format!("CFLAGS={}", ["-Dgetenv=rust_getenv",    // Hack to make the tz C code use rust's getenv (so that it is locked properly)
                                   "-DTHREAD_SAFE",           // Make tz protect shared globals with a mutex
                                   "-DSTD_INSPIRED=1",        // Add posix2time_z() and time2posix_z().
                                   "-DHAVE_TZNAME=0",         // Don't export variables--they're inherently racey
                                   "-DUSG_COMPAT=0",          // " " "
                                   "-DALTZONE=0",             // " " "
                                  ].join(" ")))
        .current_dir(std::path::Path::new("tz"));
    println!("command: {:?}", make);
    match make.status().expect("Make failed").code().expect("Make crashed?") {
        0 => {},
        e => { panic!("Make exited with {}", e); },
    }
    std::fs::copy("tz/libtz.a", out_dir.join("libtz.a")).expect(&format!("Couldn't copy libtz.a to {}", out_dir.display()));

    println!("cargo:rustc-link-lib=tz");
    println!("cargo:rustc-link-search=native={}", out_dir.display());
}
