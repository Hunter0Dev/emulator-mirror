mod adb;
mod server;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::time::Duration;

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
    /// Push the scrcpy server, start it, connect, dump the first bytes.
    ///
    /// Smoke test that the full pipeline works end-to-end: adb push,
    /// adb forward, app_process spawn, TCP connect, raw H.264 read.
    Serve {
        /// Target a specific device serial. Defaults to the only ready device.
        #[arg(long)]
        serial: Option<String>,
        /// Bytes to read from the socket before exiting (proves frames flow).
        #[arg(long, default_value_t = 4096)]
        bytes: usize,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Doctor => doctor(),
        Cmd::Devices => devices(),
        Cmd::Serve { serial, bytes } => serve(serial.as_deref(), bytes),
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

    print!("[server]   ");
    match server::locate_jar() {
        Ok(p) => println!("OK  {}", p.display()),
        Err(e) => {
            println!("FAIL  {e:#}");
            std::process::exit(1);
        }
    }

    println!();
    println!("Ready. Try: mirror-pc serve");
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

fn serve(serial: Option<&str>, bytes: usize) -> Result<()> {
    let serial = adb::pick_device(serial)?;
    let jar = server::locate_jar()?;

    let mut running = server::start(&serial, &jar)?;
    let data = server::read_some(&mut running.stream, bytes, Duration::from_secs(3))?;

    println!();
    println!("Read {} bytes from socket.", data.len());
    println!("First 64 bytes (hex):");
    for (i, chunk) in data.iter().take(256).collect::<Vec<_>>().chunks(16).enumerate() {
        let hex: String = chunk.iter().map(|b| format!("{:02x} ", **b)).collect();
        print!("  {:04x}  {hex}", i * 16);
        println!();
    }

    if data.starts_with(&[0x00, 0x00, 0x00, 0x01]) || data.starts_with(&[0x00, 0x00, 0x01]) {
        println!();
        println!("Detected H.264 NAL start code — raw_stream is flowing. Pipeline works.");
    } else if data.is_empty() {
        println!();
        println!("No data received. Likely the server failed to start. Check stderr above.");
    } else {
        println!();
        println!("Got data but no H.264 NAL prefix detected. Verify scrcpy version pin.");
    }

    Ok(())
}
