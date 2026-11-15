# Eager stereo connection

Windows connects a Bluetooth headset in stages:

1. The headset and PC establish their paired Bluetooth link (ACL).
2. Audio and control services establish their own connections over that link.
3. Windows starts AVDTP/A2DP negotiation for stereo playback.
4. Windows exposes the ready audio endpoint to applications.

On the tested setup, the Bluetooth link connected promptly, but Windows waited
about 13 seconds before requesting stereo. The reason for that wait is unknown.
The headset could complete stereo setup much sooner when asked immediately.

Stereo Nudge listens for `GUID_BLUETOOTH_HCI_EVENT` on the Bluetooth radio handle.
On a new ACL connection for any checked headset, it eagerly asks the Windows
audio driver to connect stereo instead of waiting for the usual delayed request.

The request is `KSPROPERTY_ONESHOT_RECONNECT` from `KSPROPSETID_BtAudio`, sent
through `IKsControl::KsProperty` to the headset's Audio Sink driver filter.
Windows and the headset perform the negotiation. Stereo Nudge doesn't carry,
decode, or modify the audio.

## The entire fix

**Listen for the Bluetooth connection, then immediately request the stereo audio
connection.** No audio profiles need to be disabled.

There is no continuous Bluetooth scanning or connection polling. Sign-in launch
can wait briefly for the local Bluetooth/audio services to initialize; after
that, the app responds to events. A one-second timer checks only its local stop
flag and briefly retries local radio registration during startup if necessary.

## Device choices

The installer never binds the app to one device. The tray menu reads the local
Windows classic Bluetooth pairing list whenever it opens. Supported audio devices
can be checked independently, and the selected addresses are saved in the user's
settings folder. Unchecking a device prevents future eager requests without
disconnecting its current audio.

## Checking that it worked

An accepted driver request does not itself prove that audio is ready.
`IMMNotificationClient` separately records the stereo endpoint becoming active
(`STEREO STATE=1`), including the elapsed time since the Bluetooth connection.

On one WH-1000XM4 / Windows 11 setup, the usual connection took about 14.6 seconds
to expose stereo output. With eager connection enabled, one measured connection
took 2.11 seconds, with the stereo request sent after 0.12 seconds. These are
single observations on one setup, not a guarantee for every headset or driver.

## Scope

The app targets an already-paired classic Bluetooth A2DP headset and leaves
Hands-Free Telephony enabled. It does not change pairing, codecs, the default
microphone, or the default audio output. It currently listens on the first
Bluetooth radio. The tray refreshes local device data and the app can reattach
the radio listener when Windows reports hardware changes.

The pause's cause remains unknown. This is an eager connection workaround, not a
claim to repair a particular Windows timer or driver bug.
