# Runtime resource use

One connected-idle sample of installed Stereo Nudge 0.2.0 on Windows lasted 60.014 seconds,
with 60 samples taken about one second apart. The headset was already connected;
this sample does not measure connection negotiation or installer activity.

| Metric | Observed value |
| --- | ---: |
| Average working set | 13.829 MiB |
| Working set range | 13.816–13.852 MiB |
| Average private committed memory | 1.984 MiB |
| Maximum private committed memory | 1.992 MiB |
| Lifetime peak working set | 13.852 MiB |
| Additional process CPU time during sample | 0 ms reported |

Working set is resident process memory and includes shared Windows DLL pages.
Private committed memory is memory committed exclusively to the process; it is
not the same as private resident RAM. The two figures should not be added.

The CPU counter reported no increase at its available resolution. This means
negligible measured idle CPU use, not proof that the app executes no instructions.
The app uses event notifications for Bluetooth, with no periodic radio inquiry.
After startup initialization, its one-second timer only checks a local stop flag.

These figures are observations from one machine and can change with Windows,
drivers, memory pressure, and activity. Measure your own running instance with:

```powershell
.\scripts\Measure-Runtime.ps1 -DurationSeconds 60
```

The script writes raw CSV samples and a JSON summary under `logs`, which is
excluded from Git. Use `-TargetProcessId` if multiple instances are running.

Definitions: [Microsoft process memory counters](https://learn.microsoft.com/en-us/windows/win32/api/psapi/ns-psapi-process_memory_counters_ex).
