// Copyright (C) 2026 Campbell Reid
// SPDX-License-Identifier: GPL-3.0-or-later
use crate::State;
use std::sync::Arc;
use windows::{
    Win32::{
        Foundation::{E_FAIL, PROPERTYKEY},
        Media::{Audio::*, KernelStreaming::*},
        System::Com::*,
    },
    core::{Error, PCWSTR, PWSTR, Result, implement},
};
pub struct ComApartment;
impl ComApartment {
    pub fn new(kind: COINIT) -> Result<Self> {
        // COM initialization and cleanup occur on the same thread.
        unsafe {
            CoInitializeEx(None, kind).ok()?;
        }
        Ok(Self)
    }
}
impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}
#[derive(Clone)]
pub struct Target {
    pub endpoint: String,
    filter: String,
}
unsafe fn take_string(value: PWSTR) -> Result<String> {
    let result = unsafe { value.to_string() };
    unsafe {
        CoTaskMemFree(Some(value.0.cast()));
    }
    result.map_err(|_| Error::new(E_FAIL, "Invalid Windows device string"))
}
pub fn find_stereo(address: u64) -> Result<Target> {
    // Local audio enumeration only, with no Bluetooth discovery.
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let collection = enumerator.EnumAudioEndpoints(
            eRender,
            DEVICE_STATE(DEVICE_STATE_ACTIVE.0 | DEVICE_STATE_UNPLUGGED.0),
        )?;
        let address = format!("{address:012x}");
        for index in 0..collection.GetCount()? {
            let endpoint = collection.Item(index)?;
            let candidate = (|| -> Result<Target> {
                let topology: IDeviceTopology = endpoint.Activate(CLSCTX_ALL, None)?;
                let connector = topology.GetConnector(0)?;
                let filter = take_string(connector.GetDeviceIdConnectedTo()?)?;
                let normalized = filter.to_ascii_lowercase();
                if !normalized.contains(&address)
                    || !normalized.contains("0000110b-0000-1000-8000-00805f9b34fb")
                {
                    return Err(Error::new(E_FAIL, "Different audio filter"));
                }
                Ok(Target {
                    endpoint: take_string(endpoint.GetId()?)?,
                    filter,
                })
            })();
            if let Ok(target) = candidate {
                return Ok(target);
            }
        }
        Err(Error::new(
            E_FAIL,
            "No stereo endpoint found for this paired Bluetooth address",
        ))
    }
}
fn property(target: &Target, flags: u32, data: &mut [u8]) -> Result<()> {
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let filter: Vec<u16> = target.filter.encode_utf16().chain(Some(0)).collect();
        let adapter = enumerator.GetDevice(PCWSTR(filter.as_ptr()))?;
        let control: IKsControl = adapter.Activate(CLSCTX_ALL, None)?;
        let descriptor = KSIDENTIFIER {
            Anonymous: KSIDENTIFIER_0 {
                Anonymous: KSIDENTIFIER_0_0 {
                    Set: KSPROPSETID_BtAudio,
                    Id: KSPROPERTY_ONESHOT_RECONNECT.0 as u32,
                    Flags: flags,
                },
            },
        };
        let mut returned = 0;
        control.KsProperty(
            &descriptor,
            size_of::<KSIDENTIFIER>() as u32,
            if data.is_empty() {
                std::ptr::null_mut()
            } else {
                data.as_mut_ptr().cast()
            },
            data.len() as u32,
            &mut returned,
        )
    }
}
pub fn probe(target: &Target) -> Result<()> {
    property(target, KSPROPERTY_TYPE_BASICSUPPORT, &mut [0u8; 64])
}
pub fn request(target: &Target) -> Result<()> {
    property(target, KSPROPERTY_TYPE_GET, &mut [])
}
#[implement(IMMNotificationClient)]
pub struct AudioEvents {
    state: Arc<State>,
}
impl AudioEvents {
    pub(crate) fn create_callback(state: Arc<State>) -> IMMNotificationClient {
        Self { state }.into()
    }
}
#[allow(non_snake_case)]
impl IMMNotificationClient_Impl for AudioEvents_Impl {
    fn OnDeviceStateChanged(&self, id: &PCWSTR, state: DEVICE_STATE) -> Result<()> {
        let id = unsafe { id.to_string() }.unwrap_or_default();
        self.state.endpoint_changed(&id, state.0);
        Ok(())
    }
    fn OnDeviceAdded(&self, _: &PCWSTR) -> Result<()> {
        Ok(())
    }
    fn OnDeviceRemoved(&self, _: &PCWSTR) -> Result<()> {
        Ok(())
    }
    fn OnDefaultDeviceChanged(&self, _: EDataFlow, _: ERole, _: &PCWSTR) -> Result<()> {
        Ok(())
    }
    fn OnPropertyValueChanged(&self, _: &PCWSTR, _: &PROPERTYKEY) -> Result<()> {
        Ok(())
    }
}
