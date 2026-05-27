mod adb;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "mirror-pc",
    version,
    about = "Low-latency Android mirror + input forwarding (BGMI target)"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Check that adb is installed and at least one device is connected.
    Doctor,
    /// List devices visible to adb.
    Devices,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Doctor => doctor(),
        Cmd::Devices => devices(),
    }
}

fn doctor() -> Result<()> {
    println!("mirror-pc {} — doctor", env!("CARGO_PKG_VERSION"));
    println!();

    print!("[adb]      ");
    match adb::version() {
        Ok(v) => println!("OK  {v}"),
        Err(e) => {
            println!("FAIL  {e:#}");
            println!();
            println!("Fix: install Android platform-tools and add it to PATH.");
            println!("     Android Studio installs it to:");
            println!("       %LOCALAPPDATA%\\Android\\Sdk\\platform-tools");
            std::process::exit(1);
        }
    }

    print!("[devices]  ");
    let list = adb::devices()?;
    let usable: Vec<_> = list.iter().filter(|d| d.state == "device").collect();
    if usable.is_empty() {
        println!("FAIL  no devices in `device` state");
        if !list.is_empty() {
            println!();
            println!("Found devices, but not ready:");
            for d in &list {
                println!("  {}  ({})", d.serial, d.state);
            }
            println!();
            println!("Fix: plug phone via USB, enable Developer Options + USB debugging,");
            println!("     and tap 'Allow' on the RSA prompt that appears on the phone.");
        }
        std::process::exit(1);
    }
    println!("OK  {} ready", usable.len());
    for d in &usable {
        println!("           - {}", d.serial);
    }

    println!();
    println!("Ready to mirror. (mirror command coming in a future build.)");
    Ok(())
}

fn devices() -> Result<()> {
    let list = adb::devices()?;
    if list.is_empty() {
        println!("No devices connected.");
        return Ok(());
    }
    println!("{:<24} {}", "SERIAL", "STATE");
    for d in list {
        println!("{:<24} {}", d.serial, d.state);
    }
    Ok(())
}
