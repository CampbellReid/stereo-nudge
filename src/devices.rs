use crate::audio::{self, Target};
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};
use windows::{
    Win32::{Devices::Bluetooth::*, Foundation::CloseHandle},
    core::Result,
};

fn radio_ready() -> Result<()> {
    unsafe {
        let mut radio = Default::default();
        let find = BluetoothFindFirstRadio(
            &BLUETOOTH_FIND_RADIO_PARAMS {
                dwSize: size_of::<BLUETOOTH_FIND_RADIO_PARAMS>() as u32,
            },
            &mut radio,
        )?;
        let result = BluetoothFindRadioClose(find);
        let _ = CloseHandle(radio);
        result
    }
}

pub fn wait_for_target(address: u64, wait_seconds: u64) -> Result<Target> {
    let deadline = Instant::now() + Duration::from_secs(wait_seconds);
    loop {
        let result = radio_ready().and_then(|()| audio::find_stereo(address));
        match result {
            Ok(target) => return Ok(target),
            Err(error) if Instant::now() >= deadline => return Err(error),
            Err(_) => std::thread::sleep(Duration::from_millis(500)),
        }
    }
}

#[derive(Clone)]
pub struct PairedDevice {
    pub address: u64,
    pub name: String,
    pub target: Option<Target>,
}

pub fn paired_devices() -> Result<Vec<PairedDevice>> {
    let mut devices = Vec::new();
    unsafe {
        let mut radio = Default::default();
        let radio_find = BluetoothFindFirstRadio(
            &BLUETOOTH_FIND_RADIO_PARAMS {
                dwSize: size_of::<BLUETOOTH_FIND_RADIO_PARAMS>() as u32,
            },
            &mut radio,
        )?;
        BluetoothFindRadioClose(radio_find)?;
        let search = BLUETOOTH_DEVICE_SEARCH_PARAMS {
            dwSize: size_of::<BLUETOOTH_DEVICE_SEARCH_PARAMS>() as u32,
            fReturnAuthenticated: true.into(),
            fReturnRemembered: true.into(),
            fReturnConnected: true.into(),
            hRadio: radio,
            // Return cached paired devices only: never start a radio inquiry.
            ..Default::default()
        };
        let mut info = BLUETOOTH_DEVICE_INFO {
            dwSize: size_of::<BLUETOOTH_DEVICE_INFO>() as u32,
            ..Default::default()
        };
        if let Ok(find) = BluetoothFindFirstDevice(&search, &mut info) {
            loop {
                let address = info.Address.Anonymous.ullLong;
                if info.fAuthenticated.as_bool() {
                    let length = info
                        .szName
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(info.szName.len());
                    let name = String::from_utf16_lossy(&info.szName[..length])
                        .replace(['\t', '\r', '\n'], " ");
                    devices.push(PairedDevice {
                        address,
                        name,
                        target: audio::find_stereo(address).ok(),
                    });
                }
                info.dwSize = size_of::<BLUETOOTH_DEVICE_INFO>() as u32;
                if BluetoothFindNextDevice(find, &mut info).is_err() {
                    break;
                }
            }
            BluetoothFindDeviceClose(find)?;
        }
        CloseHandle(radio)?;
    }
    devices.sort_by_key(|device| device.name.to_lowercase());
    Ok(devices)
}

pub fn write_paired_headphones(path: &Path) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let rows: String = paired_devices()?
        .iter()
        .filter(|device| device.target.is_some())
        .map(|device| format!("{:012X}\t{}\r\n", device.address, device.name))
        .collect();
    let mut bytes = vec![0xef, 0xbb, 0xbf];
    bytes.extend_from_slice(rows.as_bytes());
    fs::write(path, bytes)?;
    Ok(())
}
