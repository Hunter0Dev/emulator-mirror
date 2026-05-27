# Android server

This directory will hold the Android-side agent: a fork of scrcpy's `server/` module, customized for BGMI.

**Not yet scaffolded.** The next commit will:
1. Pull in scrcpy's server module (Java + C NDK).
2. Wire up Gradle to produce `mirror-server.jar` (pushed via ADB at runtime, not a regular APK).
3. Add a thin companion APK for on-phone settings UI (later).

Until then, [`android.yml`](../.github/workflows/android.yml) just verifies the directory exists.
