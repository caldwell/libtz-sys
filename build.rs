// Copyright © 2023 David Caldwell <david@porkrind.org>

fn main() {
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("no OUT_DIR from cargo"));

    for file in ["tz/localtime.c", "tz/asctime.c", "tz/difftime.c", "tz/strftime.c", "tz/tzfile.h", "tz/private.h"].into_iter() {
        println!("cargo:rerun-if-changed={}", file);
    }

    cc::Build::new()
        .file("tz/localtime.c")
        .file("tz/asctime.c")
        .file("tz/difftime.c")
        .file("tz/strftime.c")
        .include("tz")
        .warnings(false) // some of the flags we set up below generate "unused reference" warnings.
        .static_flag(true)
        .define("TZDIR",         r#""/usr/share/zoneinfo""#)      // The default from tz/Makefile
        .define("getenv",        "rust_getenv")                   // Hack to make the tz C code use rust's getenv (so that it is locked properly)
        .define("THREAD_SAFE",   None)                            // Make tz protect shared globals with a mutex
        .define("STD_INSPIRED",  "1")                             // Add posix2time_z() and time2posix_z().
        .define("time_tz",       "int64_t")                       // Force libtz to use a 64 bit time_t
        .define("HAVE_TZNAME",   "0")                             // Don't export variables--they're inherently racey
        .define("USG_COMPAT",    "0")                             // " " "
        .define("ALTZONE",       "0")                             // " " "
        .compile("tz");

    println!("cargo:rustc-link-lib=tz");
    println!("cargo:rustc-link-search=native={}", out_dir.display());
}
