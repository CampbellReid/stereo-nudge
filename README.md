# Stereo Nudge

A small native Windows tray app that requests Bluetooth stereo audio as soon as a
paired headset establishes its Bluetooth link. Written in Rust, with a static C
runtime and no .NET requirement.

## The fix

Windows first establishes a Bluetooth link to a headset, then connects its audio
services and makes the stereo output available. On the tested setup, Windows
waited about 13 seconds before requesting stereo, even though asking immediately
worked. The cause of that delay is unknown.

Stereo Nudge listens for the Bluetooth connection event and eagerly requests the
stereo audio connection. That's the whole fix. Windows still handles the actual
audio connection, and Hands-Free Telephony stays enabled.

One measured connection became ready in **2.11 seconds**, compared with about
14.6 seconds normally. Results vary with the headset, adapter and Windows drivers.
See [how it works](docs/how-it-works.md) for the API sequence and limits.

## Windows installer

Run `stereo-nudge-0.2.0-windows-x64-setup.exe` and optionally select **Run Stereo
Nudge when I sign in to Windows**. You can launch it immediately when setup
finishes. Setup does not ask you to select a device.

Click the tray icon to see Windows' paired Bluetooth devices. Check the audio
devices you want Stereo Nudge to help connect; uncheck them to stop automatic
requests. These choices are saved and restored next time the app starts.

The installer works for your Windows account without administrator privileges.
It installs into `%LOCALAPPDATA%\Programs\StereoNudge` and creates Start menu and
uninstall entries. Startup uses a normal per-user Startup shortcut; no background
PowerShell process or Windows service is required.

Change device choices in the tray at any time. Rerun setup to change the startup
checkbox. Uninstall from Windows
**Installed apps** or the Start menu. The uninstaller removes the startup shortcut;
local settings and diagnostic logs are kept in `%LOCALAPPDATA%\StereoNudge`.

## Build

Requires stable Rust and the Visual Studio C++ build tools with a Windows SDK.

```powershell
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
.\scripts\Build-Installer.ps1  # requires Inno Setup 6
```

The executable is `target\release\stereo-nudge.exe`. The x64 MSVC build statically
links its C runtime. Standard Windows system DLLs are still required.

## Run

Pair your headphones normally in Windows, launch the app, then check them in its
tray menu:

```powershell
.\stereo-nudge.exe
```

The app runs until you select **Exit**. `--seconds N` optionally limits a test run.
The portable app itself does not register startup. On first launch, no devices
are checked automatically.

The tray menu provides:

- **Paired device checkboxes**: enable eager audio per device. Unchecking leaves current audio connected.
- **Pause all automatic requests**: temporarily suspend requests for every device.
- **Connect checked devices now**: send manual requests even while automatic requests are paused.
- **Refresh paired devices**: reload the local Windows pairing list, also refreshed whenever the menu opens.
- **Exit**: remove the icon and stop listening.

Devices without a currently available classic stereo endpoint are shown disabled.
Settings and logs are stored in `%LOCALAPPDATA%\StereoNudge`.

For diagnostics, `--address AA:BB:CC:DD:EE:FF --probe` checks reconnect support
without requesting a connection. `--request` with an address sends one request
and exits. `--address` during a normal launch can preselect a device; the installer
and startup shortcut never force an address.
`--startup-wait-seconds 60` allows a bounded wait for local Bluetooth and audio
services to initialize at sign-in; it does not scan for powered-off headphones.

## Runtime statistics

One 60-second connected-idle sample averaged **13.83 MiB working set** and
**1.98 MiB private committed memory**, with no additional CPU time reported by
the process counter during that interval. See [measurement details](docs/runtime.md).

Use the measurement script against a running instance:

```powershell
.\scripts\Measure-Runtime.ps1 -DurationSeconds 60
```

It saves CSV samples and a JSON summary in `logs`, covering working set, private
committed memory, peak working set, and CPU time. Working set includes shared
Windows DLL pages; private committed memory is a separate metric, not a private
resident-RAM measurement. CPU is reported both relative to one core and relative
to all logical processors.

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

Requests run on COM-initialized worker threads, with at most one request per device
in flight. There is no Bluetooth discovery loop or periodic connection attempt.
The one-second timer checks its local stop flag and, briefly at startup, whether
the local radio is available so event notifications can be registered.

The app does not change pairing, audio profiles, codecs, the default microphone,
or the default output. It does not process audio or collect network telemetry.

## Logs and limits

Logs contain Unix timestamps in milliseconds, local endpoint IDs and connection
timings. Keep them private if those identifiers matter to you. Startup errors are
also written to `%TEMP%\stereo-nudge-error.txt` because release builds have no console.

The app listens on the first Bluetooth radio and uses the local classic Bluetooth
pairing list. It supports classic Bluetooth A2DP audio, not LE Audio-only devices.
Opening the tray refreshes devices after pairing changes. Bluetooth hardware
change notifications also allow the radio listener to be reattached.

To stop from a script, create the `stop-<PID>.flag` file named in the log inside
the log directory, or use the tray's **Exit** command.

## Windows API references

- [Bluetooth connection notifications](https://learn.microsoft.com/en-us/windows/win32/bluetooth/bluetooth-and-wm-devicechange-messages)
- [KSPROPERTY_ONESHOT_RECONNECT](https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/ksproperty-oneshot-reconnect)
- [Microsoft's Rust Windows bindings](https://github.com/microsoft/windows-rs)
