use std::io;
use std::path::{Path, PathBuf};

#[cfg(feature = "build_server")]
use std::process::Command;

fn main() -> io::Result<()> {
    let bundle_dir = Path::new("include/ClockKit");

    let ckfiles = vec![
        "bridge.cpp",
        "ClockPacket.cpp",
        "ClockClient.cpp",
        "ClockServer.cpp",
        "PhaseLockedClock.cpp",
        "SystemClock.cpp",
        "Timestamp.cpp",
        "VariableFrequencyClock.cpp",
    ]
    .iter()
    .map(|f| bundle_dir.join(f))
    .collect::<Vec<PathBuf>>();

    cxx_build::bridge("src/lib.rs")
        .files(ckfiles)
        .cpp(true)
        .flag("--std=c++17")
        .flag("-static")
        .warnings(false)
        .extra_warnings(false)
        .compile("libclockkit.a");

    // Build the server for testing, unfortunately this clutters the src directory with object
    // files
    #[cfg(feature = "build_server")]
    {
        Command::new("make")
            .arg("ckserver")
            .current_dir(&bundle_dir)
            .status()
            .unwrap();
    }

    println!("cargo:rustc-link-search=native={}", bundle_dir.display());
    println!("cargo:rustc-link-lib=static=clockkit");

    // Add dynamically linked libraries clockkit depends on
    println!("cargo:rustc-flags=-l dylib=stdc++");
    println!("cargo:rustc-flags=-l dylib=pthread");
    println!("cargo:rustc-flags=-l dylib=dl");
    Ok(())
}
