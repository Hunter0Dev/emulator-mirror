use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::{Child, Command, Stdio};

pub fn binary() -> String {
    std::env::var("MIRROR_ADB").unwrap_or_else(|_| "adb".to_string())
}

fn cmd() -> Command {
    Command::new(binary())
}

pub fn version() -> Result<String> {
    let out = cmd()
        .arg("version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("failed to run `{} version` — is adb on PATH?", binary()))?;

    if !out.status.success() {
        bail!(
            "adb version exited {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }

    let stdout = String::from_utf8_lossy(&out.stdout);
    Ok(stdout.lines().next().unwrap_or("").trim().to_string())
}

#[derive(Debug, Clone)]
pub struct Device {
    pub serial: String,
    pub state: String,
}

pub fn devices() -> Result<Vec<Device>> {
    let out = cmd()
        .arg("devices")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("failed to run `adb devices`")?;

    if !out.status.success() {
        bail!(
            "adb devices exited {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }

    let stdout = String::from_utf8_lossy(&out.stdout);
    let devices = stdout
        .lines()
        .skip(1)
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            let mut parts = line.split_whitespace();
            let serial = parts.next()?.to_string();
            let state = parts.next().unwrap_or("unknown").to_string();
            Some(Device { serial, state })
        })
        .collect();

    Ok(devices)
}

/// Pick a single ready device, or error with guidance.
pub fn pick_device(explicit: Option<&str>) -> Result<String> {
    let list = devices()?;
    let ready: Vec<_> = list.into_iter().filter(|d| d.state == "device").collect();

    if let Some(want) = explicit {
        if ready.iter().any(|d| d.serial == want) {
            return Ok(want.to_string());
        }
        bail!("device {want} not connected or not in `device` state");
    }

    match ready.as_slice() {
        [] => bail!("no devices ready — run `mirror-pc doctor` for details"),
        [only] => Ok(only.serial.clone()),
        many => {
            let serials: Vec<_> = many.iter().map(|d| d.serial.as_str()).collect();
            bail!(
                "multiple devices connected ({}). Pass --serial <SERIAL>.",
                serials.join(", ")
            )
        }
    }
}

fn check_status(out: std::process::Output, what: &str) -> Result<()> {
    if !out.status.success() {
        bail!(
            "{what} failed ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

pub fn push(serial: &str, local: &Path, remote: &str) -> Result<()> {
    let out = cmd()
        .args(["-s", serial, "push"])
        .arg(local)
        .arg(remote)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("failed to run `adb push {} {remote}`", local.display()))?;
    check_status(out, "adb push")
}

pub fn forward(serial: &str, local_port: u16, remote_socket: &str) -> Result<()> {
    let out = cmd()
        .args([
            "-s",
            serial,
            "forward",
            &format!("tcp:{local_port}"),
            remote_socket,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("failed to run `adb forward`")?;
    check_status(out, "adb forward")
}

pub fn forward_remove(serial: &str, local_port: u16) -> Result<()> {
    let out = cmd()
        .args([
            "-s",
            serial,
            "forward",
            "--remove",
            &format!("tcp:{local_port}"),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("failed to run `adb forward --remove`")?;
    check_status(out, "adb forward --remove")
}

/// Spawn a long-running `adb shell` process. Caller is responsible for killing it.
pub fn shell_spawn(serial: &str, args: &[&str]) -> Result<Child> {
    cmd()
        .args(["-s", serial, "shell"])
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to spawn `adb shell`")
}
