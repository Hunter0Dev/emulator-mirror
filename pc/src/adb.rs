use anyhow::{bail, Context, Result};
use std::process::{Command, Stdio};

pub fn binary() -> String {
    std::env::var("MIRROR_ADB").unwrap_or_else(|_| "adb".to_string())
}

pub fn version() -> Result<String> {
    let out = Command::new(binary())
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
    let first = stdout.lines().next().unwrap_or("").trim().to_string();
    Ok(first)
}

#[derive(Debug, Clone)]
pub struct Device {
    pub serial: String,
    pub state: String,
}

pub fn devices() -> Result<Vec<Device>> {
    let out = Command::new(binary())
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
