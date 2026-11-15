use crate::{STATE, State, audio};
use std::{
    ffi::c_void,
    mem::{offset_of, size_of},
    sync::{Arc, atomic::Ordering},
    time::Instant,
};
use windows::{
    Win32::{
        Devices::Bluetooth::*,
        Foundation::*,
        Media::Audio::*,
        System::{Com::*, LibraryLoader::*, Threading::*},
        UI::{Shell::*, WindowsAndMessaging::*},
    },
    core::{Error, GUID, Result, w},
};
const HCI_EVENT: GUID = GUID::from_u128(0xfc240062_1541_49be_b463_84c4dcd7bf7f);
const TRAY_MESSAGE: u32 = WM_APP + 1;

fn parse_hci(bytes: &[u8]) -> Option<(u64, u8, bool)> {
    let data_offset = offset_of!(DEV_BROADCAST_HANDLE, dbch_data);
    let guid_offset = offset_of!(DEV_BROADCAST_HANDLE, dbch_eventguid);
    if bytes.len() < data_offset + 10 {
        return None;
    }
    let declared = u32::from_le_bytes(bytes.get(..4)?.try_into().ok()?) as usize;
    let device_type = u32::from_le_bytes(bytes.get(4..8)?.try_into().ok()?);
    if declared > bytes.len() || declared < data_offset + 10 || device_type != DBT_DEVTYP_HANDLE.0 {
        return None;
    }
    // The variable-length payload isn't necessarily aligned for Rust's GUID/u64 types.
    let guid = unsafe {
        bytes
            .as_ptr()
            .add(guid_offset)
            .cast::<GUID>()
            .read_unaligned()
    };
    if guid != HCI_EVENT {
        return None;
    }
    let data = &bytes[data_offset..];
    Some((
        u64::from_le_bytes(data[..8].try_into().ok()?),
        data[8],
        data[9] != 0,
    ))
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // Windows supplies the notification buffer. Bounds-check its size before parsing.
    unsafe {
        if let Some(state) = STATE.get() {
            match message {
                WM_DEVICECHANGE if wparam.0 == DBT_CUSTOMEVENT as usize && lparam.0 != 0 => {
                    let pointer = lparam.0 as *const u8;
                    let length = pointer.cast::<u32>().read_unaligned() as usize;
                    if (12..=65536).contains(&length)
                        && let Some((address, kind, connected)) =
                            parse_hci(std::slice::from_raw_parts(pointer, length))
                        && address == state.address
                        && kind == 1
                    {
                        state.log(&format!("HCI ACL: connected={connected}"));
                        let was_linked = state.linked.swap(connected, Ordering::AcqRel);
                        if connected && !was_linked {
                            if let Ok(mut time) = state.connected_at.lock() {
                                *time = Some(Instant::now());
                            }
                            state.request(false);
                        }
                    }
                }
                TRAY_MESSAGE
                    if lparam.0 as u32 == WM_RBUTTONUP || lparam.0 as u32 == WM_LBUTTONUP =>
                {
                    if let Ok(menu) = CreatePopupMenu() {
                        let checked = if state.enabled.load(Ordering::Acquire) {
                            MF_CHECKED
                        } else {
                            MF_UNCHECKED
                        };
                        let _ =
                            AppendMenuW(menu, MF_STRING | checked, 1, w!("Early stereo enabled"));
                        let _ = AppendMenuW(menu, MF_STRING, 2, w!("Connect stereo now"));
                        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
                        let _ = AppendMenuW(menu, MF_STRING, 3, w!("Exit"));
                        let mut point = POINT::default();
                        let _ = GetCursorPos(&mut point);
                        let _ = SetForegroundWindow(hwnd);
                        let command = TrackPopupMenu(
                            menu,
                            TPM_RETURNCMD | TPM_RIGHTBUTTON,
                            point.x,
                            point.y,
                            None,
                            hwnd,
                            None,
                        )
                        .0;
                        let _ = DestroyMenu(menu);
                        match command {
                            1 => {
                                let old = state.enabled.fetch_xor(true, Ordering::AcqRel);
                                state.log(&format!("ENABLED={}", !old));
                            }
                            2 => state.request(true),
                            3 => {
                                let _ = DestroyWindow(hwnd);
                            }
                            _ => {}
                        }
                        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
                    }
                }
                WM_TIMER => {
                    // Stop flag only; no Bluetooth polling or discovery.
                    if state.stop_path.exists() {
                        let _ = DestroyWindow(hwnd);
                    }
                }
                WM_CLOSE => {
                    let _ = DestroyWindow(hwnd);
                }
                WM_DESTROY => {
                    PostQuitMessage(0);
                }
                _ => {}
            }
        }
        DefWindowProcW(hwnd, message, wparam, lparam)
    }
}

pub fn run(state: &Arc<State>) -> Result<()> {
    unsafe {
        let name: Vec<u16> = format!("Local\\SonyStereoNudge_{:012X}\0", state.address)
            .encode_utf16()
            .collect();
        let mutex = CreateMutexW(None, false, windows::core::PCWSTR(name.as_ptr()))?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let _ = CloseHandle(mutex);
            return Err(Error::new(
                E_FAIL,
                "Another stereo helper is already running",
            ));
        }
        let mut radio = HANDLE::default();
        let find = BluetoothFindFirstRadio(
            &BLUETOOTH_FIND_RADIO_PARAMS {
                dwSize: size_of::<BLUETOOTH_FIND_RADIO_PARAMS>() as u32,
            },
            &mut radio,
        )?;
        BluetoothFindRadioClose(find)?;
        let module = GetModuleHandleW(None)?;
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: module.into(),
            lpszClassName: w!("StereoNudgeRustListener"),
            ..Default::default()
        };
        if RegisterClassW(&class) == 0 {
            return Err(Error::from_thread());
        }
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class.lpszClassName,
            w!("Stereo Nudge"),
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            None,
            None,
            Some(module.into()),
            None,
        )?;
        let filter = DEV_BROADCAST_HANDLE {
            dbch_size: size_of::<DEV_BROADCAST_HANDLE>() as u32,
            dbch_devicetype: DBT_DEVTYP_HANDLE.0,
            dbch_handle: radio,
            ..Default::default()
        };
        let notification = RegisterDeviceNotificationW(
            HANDLE(hwnd.0),
            (&filter as *const DEV_BROADCAST_HANDLE).cast::<c_void>(),
            DEVICE_NOTIFY_WINDOW_HANDLE,
        )?;
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let callback = audio::AudioEvents::create_callback(state.clone());
        enumerator.RegisterEndpointNotificationCallback(&callback)?;
        let mut icon = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: TRAY_MESSAGE,
            hIcon: LoadIconW(None, IDI_INFORMATION)?,
            ..Default::default()
        };
        let tip: Vec<u16> = "Stereo Nudge - native Rust trial\0"
            .encode_utf16()
            .collect();
        icon.szTip[..tip.len()].copy_from_slice(&tip);
        if !Shell_NotifyIconW(NIM_ADD, &icon).as_bool() {
            return Err(Error::from_thread());
        }
        SetTimer(Some(hwnd), 1, 1000, None);
        if state.seconds > 0 {
            let window_value = hwnd.0 as usize;
            let seconds = state.seconds;
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(seconds));
                let _ = PostMessageW(
                    Some(HWND(window_value as *mut c_void)),
                    WM_CLOSE,
                    WPARAM(0),
                    LPARAM(0),
                );
            });
        }
        state.log(&format!(
            "ARMED: event-driven Rust tray helper; seconds={}; stop_flag={}",
            state.seconds,
            state.stop_path.display()
        ));
        let mut message = MSG::default();
        loop {
            let status = GetMessageW(&mut message, None, 0, 0).0;
            if status <= 0 {
                break;
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        state.enabled.store(false, Ordering::Release);
        let _ = KillTimer(Some(hwnd), 1);
        let _ = Shell_NotifyIconW(NIM_DELETE, &icon);
        let _ = enumerator.UnregisterEndpointNotificationCallback(&callback);
        let _ = UnregisterDeviceNotification(notification);
        let _ = CloseHandle(radio);
        let _ = CloseHandle(mutex);
        state.log("STOPPED");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn packet() -> Vec<u8> {
        let offset = offset_of!(DEV_BROADCAST_HANDLE, dbch_data);
        let mut bytes = vec![0u8; offset + 10];
        let len = bytes.len() as u32;
        bytes[..4].copy_from_slice(&len.to_le_bytes());
        bytes[4..8].copy_from_slice(&DBT_DEVTYP_HANDLE.0.to_le_bytes());
        unsafe {
            bytes
                .as_mut_ptr()
                .add(offset_of!(DEV_BROADCAST_HANDLE, dbch_eventguid))
                .cast::<GUID>()
                .write_unaligned(HCI_EVENT);
        }
        bytes[offset..offset + 8].copy_from_slice(&0xaabbccddeeffu64.to_le_bytes());
        bytes[offset + 8] = 1;
        bytes[offset + 9] = 32; // Connection status is nonzero, not necessarily 1.
        bytes
    }
    #[test]
    fn accepts_nonzero_connection_status() {
        assert_eq!(parse_hci(&packet()), Some((0xaabbccddeeff, 1, true)));
    }
    #[test]
    fn rejects_truncated_and_unrelated_notifications() {
        let mut bytes = packet();
        for size in 0..bytes.len() {
            assert!(parse_hci(&bytes[..size]).is_none());
        }
        bytes[4..8].copy_from_slice(&0u32.to_le_bytes());
        assert!(parse_hci(&bytes).is_none());
    }
}
