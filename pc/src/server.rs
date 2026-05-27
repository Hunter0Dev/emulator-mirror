//! scrcpy server lifecycle: push the jar, set up the forward, spawn it, connect.
//!
//! We pin scrcpy v4.0 and start the server in `raw_stream=true` mode, which
//! emits raw H.264 NAL units on the socket (no scrcpy framing protocol). This
//! is the simplest input for our FFmpeg decoder in a future commit.

use crate::adb;
use anyhow::{bail, Context, Result};
use std::io::Read;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::Child;
use std::thread;
use std::time::{Duration, Instant};

pub const SCRCPY_VERSION: &str = "4.0";
const REMOTE_JAR_PATH: &str = "/data/local/tmp/scrcpy-server.jar";
const ABSTRACT_SOCKET: &str = "localabstract:scrcpy";
const DEFAULT_LOCAL_PORT: u16 = 27184;

/// Find the server jar locally. Checks `MIRROR_SERVER_JAR` env var first,
/// then `android/prebuilt/scrcpy-server-v<VERSION>` relative to CWD.
pub fn locate_jar() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("MIRROR_SERVER_JAR") {
        let path = PathBuf::from(&p);
        if path.exists() {
            return Ok(path);
        }
        bail!("MIRROR_SERVER_JAR points to {p} but file does not exist");
    }

    let rel = PathBuf::from(format!("android/prebuilt/scrcpy-server-v{SCRCPY_VERSION}"));
    if rel.exists() {
        return Ok(rel);
    }

    bail!(
        "scrcpy server jar not found.\n\
         Expected at:\n  {}\n\
         Fix: run scripts/fetch-scrcpy-server.ps1 from the repo root, or set\n\
              MIRROR_SERVER_JAR=<path> to point at the file.",
        rel.display()
    )
}

/// RAII guard that removes the adb forward when dropped.
pub struct ForwardGuard {
    serial: String,
    port: u16,
}

impl Drop for ForwardGuard {
    fn drop(&mut self) {
        let _ = adb::forward_remove(&self.serial, self.port);
    }
}

/// RAII guard that kills the child adb-shell (and the server it spawned) on drop.
pub struct ServerProcess {
    child: Option<Child>,
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

pub struct RunningServer {
    pub stream: TcpStream,
    // Order matters: stream is dropped first, then process, then forward.
    pub _process: ServerProcess,
    pub _forward: ForwardGuard,
}

pub fn start(serial: &str, local_jar: &Path) -> Result<RunningServer> {
    println!("[server] pushing {} -> {REMOTE_JAR_PATH}", local_jar.display());
    adb::push(serial, local_jar, REMOTE_JAR_PATH)?;

    let port = DEFAULT_LOCAL_PORT;
    println!("[server] adb forward tcp:{port} {ABSTRACT_SOCKET}");
    adb::forward(serial, port, ABSTRACT_SOCKET)?;
    let forward = ForwardGuard {
        serial: serial.to_string(),
        port,
    };

    let args = [
        &format!("CLASSPATH={REMOTE_JAR_PATH}"),
        "app_process",
        "/",
        "com.genymobile.scrcpy.Server",
        SCRCPY_VERSION,
        "tunnel_forward=true",
        "audio=false",
        "control=false",
        "cleanup=false",
        "raw_stream=true",
        "max_size=1920",
    ];
    let args: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();

    println!("[server] launching app_process com.genymobile.scrcpy.Server {SCRCPY_VERSION} ...");
    let child = adb::shell_spawn(serial, &args)?;
    let process = ServerProcess { child: Some(child) };

    let stream = connect_with_retry(port, Duration::from_secs(5))?;
    println!("[server] connected on localhost:{port}");

    Ok(RunningServer {
        stream,
        _process: process,
        _forward: forward,
    })
}

fn connect_with_retry(port: u16, total: Duration) -> Result<TcpStream> {
    let deadline = Instant::now() + total;
    let mut last_err = None;
    while Instant::now() < deadline {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(s) => return Ok(s),
            Err(e) => {
                last_err = Some(e);
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
    Err(anyhow::anyhow!(
        "could not connect to scrcpy server on 127.0.0.1:{port} within {total:?}: {}",
        last_err
            .map(|e| e.to_string())
            .unwrap_or_else(|| "no error".into())
    ))
    .context("did the server crash on launch? check stderr above")
}

/// Read up to `max` bytes from the stream (with a read timeout) and return them.
/// Used for the smoke-test serve command — proves data is flowing.
pub fn read_some(stream: &mut TcpStream, max: usize, timeout: Duration) -> Result<Vec<u8>> {
    stream.set_read_timeout(Some(timeout))?;
    let mut buf = vec![0u8; max];
    let mut got = 0;
    let deadline = Instant::now() + timeout;
    while got < max && Instant::now() < deadline {
        match stream.read(&mut buf[got..]) {
            Ok(0) => break,
            Ok(n) => got += n,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => break,
            Err(e) => return Err(e).context("read from scrcpy socket"),
        }
    }
    buf.truncate(got);
    Ok(buf)
}
