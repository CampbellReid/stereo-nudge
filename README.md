# Stereo Nudge

Get your Bluetooth headphones ready for audio sooner on Windows.

## Is this the problem you have?

Your headphones connect over Bluetooth, but take several more seconds to appear
in Windows' sound outputs. Stereo Nudge asks Windows to connect stereo audio as
soon as the Bluetooth link is ready, cutting out the wait we observed before
Windows started that step.

Tested with **Sony WH-1000XM4 headphones, Windows 11 and an ASUS USB-BT500 adapter**.
Other classic Bluetooth headphones may also work. Supports Windows 10/11 on x64;
LE Audio-only devices are not supported.

## How much faster?

On the tested setup, audio became available in **2.1 seconds instead of 14.6 seconds**.
That's one measured comparison; results vary with your headphones and drivers.

## Install

1. **[Download the Windows installer](https://github.com/CampbellReid/stereo-nudge/releases/latest/download/stereo-nudge-windows-x64-setup.exe)** and run it.
2. Optionally check **Run Stereo Nudge when I sign in to Windows**, then launch the app.
3. Click its system tray icon and check your headphones in the paired-device list.

Pair your headphones in Windows first. Your choices are saved automatically.
Leave Stereo Nudge running in the tray for future connections.

To upgrade, download and run the latest installer. To uninstall, use Windows'
**Installed apps**.

## More details

- [How it works and measured connection timings](docs/how-it-works.md)
- [Usage, troubleshooting and technical reference](docs/reference.md)
- [Building from source and releases](docs/releases.md)
- [Memory and CPU measurements](docs/runtime.md)
- [All downloads](https://github.com/CampbellReid/stereo-nudge/releases) · [Report an issue](https://github.com/CampbellReid/stereo-nudge/issues)

## Licence

Copyright (C) 2026 [Campbell Reid](https://github.com/CampbellReid).
Licensed under **[GPLv3-or-later](LICENSE)**. Free software, provided without warranty.
