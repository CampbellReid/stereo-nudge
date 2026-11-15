# Investigation and validation

## Observed behavior

The Sony WH-1000XM4 connected to Windows over Bluetooth promptly, but took several
more seconds to become available as a sound output. Disabling Hands-Free Telephony
made manual stereo connections faster but prevented reliable automatic reconnection
in this setup. Both services were restored before testing Stereo Nudge.

These trials used one Windows 11 PC, a USB Bluetooth adapter, and a Sony
WH-1000XM4. Host names, Bluetooth addresses, endpoint identifiers, and exact local
OS/driver inventories are excluded from this repository.

## Capture method

Bluetooth stack traces were collected with Windows Performance Recorder and
[Microsoft's BluetoothStack.wprp profile](https://github.com/microsoft/busiotools/tree/master/bluetooth/tracing).
[Microsoft BTETLParse](https://learn.microsoft.com/en-us/windows-hardware/drivers/bluetooth/testing-btp-tools-btetlparse)
exported the HCI packets for analysis. Raw traces and local device identifiers are
not included in this repository.

The baseline showed an incoming connection initiated by the headset, followed by
a successful ACL connection and a short hands-free command exchange. Windows did
not send its first AVDTP L2CAP connection request until 13.41 seconds after the ACL
link completed. The stereo endpoint became active about 1.19 seconds afterward.

The capture establishes where the pause occurs, but not the Windows internal
reason for deferring AVDTP setup. No particular timeout, driver defect, or firmware
bug has been proven.

## Results

| Trial | ACL connected to first PC AVDTP request | ACL connected to stereo output active |
| --- | ---: | ---: |
| Windows alone | 13.41 s | 14.61 s |
| C# proof of concept | 0.15 s | 1.42 s |
| Rust port | 0.12 s | 2.11 s |

These were separate single power-cycle trials, not a controlled benchmark. For
the helpers, endpoint-ready timing comes from the local connection notification
and the audio endpoint state callback; baseline timing comes from the HCI and
Windows audio event timestamps. Both helpers left stereo and Hands-Free Telephony
enabled, and only one helper ran during each trial.

The request return code is logged separately from `STEREO STATE=1`. A successful
`KsProperty` call means the driver accepted the request, not that the headset is
already usable. `IMMNotificationClient` records actual audio endpoint readiness.

## Validation performed on the Rust port

- Notification parser tests cover truncated buffers, unrelated device notifications,
  unaligned variable payloads, and a connected flag whose value is nonzero but not 1.
- Address parser tests cover common address formats and malformed input.
- `cargo test`, `cargo fmt --check`, and strict Clippy checks pass.
- A read-only probe confirmed that the selected Audio Sink filter supports the
  reconnect property.
- A one-shot request returned success on the real driver.
- A short tray listener trial registered callbacks and shut down cleanly.
- An actual headphone power cycle produced the 2.11-second Rust result above.
- Binary dependency inspection found only Windows system DLL dependencies, with
  neither .NET nor a dynamically linked Visual C++ runtime required.

The tray toggle and manual menu command are present; exhaustive GUI and hardware
compatibility testing has not been performed. The current app listens on the first
radio and requires a restart after radio replacement or re-pairing the device.
