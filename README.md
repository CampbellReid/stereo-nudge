# Stereo Nudge

A small native Windows tray app that requests Bluetooth stereo audio as soon as a
paired headset establishes its Bluetooth link. Written in Rust, with a static C
runtime and no .NET requirement.

## Why it exists

A WH-1000XM4 connection trace showed a successful Bluetooth connection followed by
13.4 seconds before Windows initiated AVDTP stereo setup. A C# proof of concept
sent a stereo reconnect request on the Bluetooth connection event: Windows
initiated stereo within 0.15 seconds and exposed the output after about 1.4 seconds.
Both stereo and Hands-Free Telephony remained enabled.

The native Rust port was then tested on the same setup and exposed stereo audio
in **2.11 seconds**, also with Hands-Free Telephony enabled. These are individual
trials on one WH-1000XM4 / USB Bluetooth adapter / Windows 11 setup, not an average or a
guarantee for other hardware.

The tested x64 release is approximately **307 KiB**, with only Windows system DLL
dependencies. See [the investigation](docs/investigation.md) for the evidence and
the distinction between requesting a connection and confirming audio is ready.

## Build

Requires stable Rust and the Visual Studio C++ build tools with a Windows SDK.

```powershell
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
```

The executable is `target\release\stereo-nudge.exe`. The x64 MSVC build statically
links its C runtime. Standard Windows system DLLs are still required.

## Run

Pair the headset normally in Windows first, then supply its Bluetooth address:

```powershell
.\stereo-nudge.exe --address AA:BB:CC:DD:EE:FF --seconds 600 --log-dir .\logs
```

By default, the trial exits after ten minutes. `--seconds 0` keeps it running
until you select **Exit**. Nothing is registered to launch at login.

The tray menu provides:

- **Early stereo enabled**: toggle automatic requests. Pausing leaves audio connected.
- **Connect stereo now**: send one manual request, even while automatic requests are paused.
- **Exit**: remove the icon and stop listening.

`--probe` discovers the stereo endpoint and checks reconnect support without
requesting a connection. `--request` sends one connection request and exits.

## How it works

1. Resolve the target address to a local A2DP audio endpoint. Its kernel-streaming
   filter must contain the target address and the Audio Sink service UUID `110b`.
2. Register for `WM_DEVICECHANGE` / `GUID_BLUETOOTH_HCI_EVENT` notifications on the
   Bluetooth radio handle.
3. On a disconnected-to-connected ACL transition for that address, send
   `KSPROPERTY_ONESHOT_RECONNECT` from `KSPROPSETID_BtAudio` through `IKsControl`.
4. Observe the stereo endpoint state with `IMMNotificationClient`, recording when
   it actually becomes active. A successful reconnect call only means the request
   was accepted; it does not prove audio is connected.

Requests run on a COM-initialized worker thread, with at most one request in
flight. There is no Bluetooth discovery loop or periodic connection attempt.
The trial's one-second timer only checks its local stop flag.

The app does not change pairing, audio profiles, codecs, the default microphone,
or the default output. It does not process audio or collect network telemetry.

## Logs and limits

Logs contain Unix timestamps in milliseconds, local endpoint IDs and connection
timings. Keep them private if those identifiers matter to you. Startup errors are
also written to `%TEMP%\stereo-nudge-error.txt` because release builds have no console.

The current trial listens on the first Bluetooth radio. Restart the app after
removing/replacing that adapter or re-pairing the headset, so it can resolve the
new endpoint and register a fresh radio handle. It supports classic Bluetooth
A2DP audio, not LE Audio-only devices.

To stop from a script, create the `stop-<PID>.flag` file named in the log inside
the log directory, or use the tray's **Exit** command.

## Windows API references

- [Bluetooth connection notifications](https://learn.microsoft.com/en-us/windows/win32/bluetooth/bluetooth-and-wm-devicechange-messages)
- [KSPROPERTY_ONESHOT_RECONNECT](https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/ksproperty-oneshot-reconnect)
- [Microsoft's Rust Windows bindings](https://github.com/microsoft/windows-rs)
