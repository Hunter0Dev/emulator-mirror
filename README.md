# Mirror

Low-latency screen-mirror + keyboard/mouse → touch input forwarding for Android, targeted at BGMI.

Phone runs the game. PC mirrors the display and sends keyboard/mouse events back as touch events. Because the game actually runs on the phone, this sidesteps emulator-detection bans.

## Architecture (v1, USB-only)

```
[ PC client (Rust) ]                       [ Android phone ]
   keyboard/mouse  ─────── ADB socket ──→  input injection (UInput)
   D3D11 swapchain ←────── ADB socket ───  MediaProjection + MediaCodec
```

- **PC side:** Rust binary. Hardware H.264 decode via FFmpeg (NVDEC/D3D11VA), direct render to D3D11 swapchain (flip-discard, allow-tearing, VSync off).
- **Android side:** scrcpy server fork. MediaCodec NDK for capture/encode, `app_process` for UInput injection without root.
- **Transport:** ADB over USB. `adb forward` exposes a TCP socket; raw framed protocol over it.
- **Target latency:** 30–50ms glass-to-glass.

## Repo layout

```
pc/                    Rust binary — the PC client (our product)
android/               scrcpy server fork — TBD next commit
scripts/               adb helpers, latency measurement
.github/workflows/     CI: builds windows .exe + android .apk artifacts
```

## Toolchain setup

You need these installed locally to develop. CI builds the same way on GitHub-hosted runners.

| Tool | Version | Where |
|---|---|---|
| Rust (cargo) | stable (≥1.80) | https://rustup.rs |
| JDK | 17 (Temurin) | https://adoptium.net |
| Android Studio | latest | https://developer.android.com/studio (bundles SDK + ADB + NDK) |
| Git | any | https://git-scm.com |

After installing Android Studio, ensure `adb` is on `PATH`:
- Add `%LOCALAPPDATA%\Android\Sdk\platform-tools` to your Windows `PATH`.
- Verify: `adb version`.

## Build locally

```powershell
# PC client
cargo build --release
# → target/release/mirror-pc.exe

# Android server (once scaffolded in next commit)
# cd android && ./gradlew assembleRelease
```

## CI

Pushes to `main` and tags trigger GitHub Actions:
- `windows.yml` → `mirror-pc.exe` artifact
- `android.yml` → `mirror-server.apk` artifact (stub for now)

Download from the Actions run page, or from a Release if a tag was pushed.

## Status

Day-0 scaffold. PC binary currently prints a stub message; no mirroring yet. Next: fork scrcpy server into `android/`, then wire transport.
