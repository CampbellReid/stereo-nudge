// Copyright (C) 2026 Campbell Reid
// SPDX-License-Identifier: GPL-3.0-or-later
use crate::{STATE, State, audio};
use std::{
    ffi::c_void,
    mem::{offset_of, size_of},
    sync::{Arc, atomic::Ordering},
};
use windows::{
    Win32::{
        Devices::Bluetooth::*,
        Foundation::*,
        Media::Audio::*,
        System::{Com::*, LibraryLoader::*, Threading::*},
        UI::{Shell::*, WindowsAndMessaging::*},
    },
    core::{Error, GUID, PCWSTR, Result, w},
};
const HCI_EVENT: GUID = GUID::from_u128(0xfc240062_1541_49be_b463_84c4dcd7bf7f);
const TRAY_MESSAGE: u32 = WM_APP + 1;

pub(crate) struct RadioBinding {
    radio: usize,
    notification: usize,
}
impl Drop for RadioBinding {
    fn drop(&mut self) {
        // These owned Windows handles may be closed from any thread.
        unsafe {
            let _ = UnregisterDeviceNotification(HDEVNOTIFY(self.notification as *mut c_void));
            let _ = CloseHandle(HANDLE(self.radio as *mut c_void));
        }
    }
}

fn ensure_radio(state: &State, hwnd: HWND) {
    if state
        .radio
        .lock()
        .map(|binding| binding.is_some())
        .unwrap_or(true)
    {
        return;
    }
    let result = (|| -> Result<RadioBinding> {
        unsafe {
            let mut radio = HANDLE::default();
            let find = BluetoothFindFirstRadio(
                &BLUETOOTH_FIND_RADIO_PARAMS {
                    dwSize: size_of::<BLUETOOTH_FIND_RADIO_PARAMS>() as u32,
                },
                &mut radio,
            )?;
            BluetoothFindRadioClose(find)?;
            let filter = DEV_BROADCAST_HANDLE {
                dbch_size: size_of::<DEV_BROADCAST_HANDLE>() as u32,
                dbch_devicetype: DBT_DEVTYP_HANDLE.0,
                dbch_handle: radio,
                ..Default::default()
            };
            match RegisterDeviceNotificationW(
                HANDLE(hwnd.0),
                (&filter as *const DEV_BROADCAST_HANDLE).cast(),
                DEVICE_NOTIFY_WINDOW_HANDLE,
            ) {
                Ok(notification) => Ok(RadioBinding {
                    radio: radio.0 as usize,
                    notification: notification.0 as usize,
                }),
                Err(error) => {
                    let _ = CloseHandle(radio);
                    Err(error)
                }
            }
        }
    })();
    // Keep the tray available even when Bluetooth isn't currently available.
    if let Ok(binding) = result {
        if let Ok(mut current) = state.radio.lock() {
            *current = Some(binding);
        }
        state.log("RADIO: connection event listener registered");
        state.refresh_devices();
    }
}

fn parse_hci(bytes: &[u8]) -> Option<(u64, u8, bool)> {
    let offset = offset_of!(DEV_BROADCAST_HANDLE, dbch_data);
    if bytes.len() < offset + 10 {
        return None;
    }
    let declared = u32::from_le_bytes(bytes.get(..4)?.try_into().ok()?) as usize;
    if declared > bytes.len()
        || declared < offset + 10
        || u32::from_le_bytes(bytes[4..8].try_into().ok()?) != DBT_DEVTYP_HANDLE.0
    {
        return None;
    }
    let guid = unsafe {
        bytes
            .as_ptr()
            .add(offset_of!(DEV_BROADCAST_HANDLE, dbch_eventguid))
            .cast::<GUID>()
            .read_unaligned()
    };
    if guid != HCI_EVENT {
        return None;
    }
    let data = &bytes[offset..];
    Some((
        u64::from_le_bytes(data[..8].try_into().ok()?),
        data[8],
        data[9] != 0,
    ))
}

unsafe fn show_menu(state: &Arc<State>, hwnd: HWND) {
    unsafe {
        ensure_radio(state, hwnd);
        state.refresh_devices();
        let devices = state
            .devices
            .lock()
            .map(|devices| devices.clone())
            .unwrap_or_default();
        let selected = state
            .selected
            .lock()
            .map(|selected| selected.clone())
            .unwrap_or_default();
        let Ok(menu) = CreatePopupMenu() else {
            return;
        };
        let _ = AppendMenuW(menu, MF_STRING | MF_GRAYED, 0, w!("Paired devices"));
        if devices.is_empty() {
            let _ = AppendMenuW(
                menu,
                MF_STRING | MF_GRAYED,
                0,
                w!("No paired devices found"),
            );
        }
        for (index, device) in devices.iter().enumerate() {
            let mut flags = MF_STRING;
            if selected.contains(&device.address) {
                flags |= MF_CHECKED;
            }
            let mut title = device.name.replace('&', "&&");
            if device.target.is_none() {
                if !selected.contains(&device.address) {
                    flags |= MF_GRAYED;
                }
                title.push_str(" (audio unavailable)");
            }
            let title: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
            let _ = AppendMenuW(menu, flags, 1000 + index, PCWSTR(title.as_ptr()));
        }
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
        let paused = if state.enabled.load(Ordering::Acquire) {
            MF_UNCHECKED
        } else {
            MF_CHECKED
        };
        let _ = AppendMenuW(
            menu,
            MF_STRING | paused,
            1,
            w!("Pause all automatic requests"),
        );
        let _ = AppendMenuW(menu, MF_STRING, 2, w!("Connect checked devices now"));
        let _ = AppendMenuW(menu, MF_STRING, 4, w!("Refresh paired devices"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
        let _ = AppendMenuW(menu, MF_STRING, 5, w!("Project on GitHub"));
        let _ = AppendMenuW(menu, MF_STRING, 6, w!("About Stereo Nudge"));
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
            2 => {
                for address in selected {
                    state.request(address, true);
                }
            }
            3 => {
                let _ = DestroyWindow(hwnd);
            }
            4 => state.refresh_devices(),
            5 => {
                let result = ShellExecuteW(
                    Some(hwnd),
                    w!("open"),
                    w!("https://github.com/CampbellReid/stereo-nudge"),
                    None,
                    None,
                    SW_SHOWNORMAL,
                );
                if result.0 as isize <= 32 {
                    state.log("Could not open the project URL in the default browser");
                }
            }
            6 => {
                let text = format!(
                    "Stereo Nudge {}\n\nCopyright (C) 2026 Campbell Reid\n\nFree software under the GNU General Public License,\nversion 3 or any later version. Comes with no warranty.\n\nSource, downloads and licence:\nhttps://github.com/CampbellReid/stereo-nudge\n\nSee LICENSE and THIRD-PARTY-NOTICES.txt in the install folder.",
                    crate::VERSION
                );
                let text: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
                let _ = MessageBoxW(
                    Some(hwnd),
                    PCWSTR(text.as_ptr()),
                    w!("About Stereo Nudge"),
                    MB_OK | MB_ICONINFORMATION,
                );
            }
            value if value >= 1000 => {
                if let Some(device) = devices.get((value - 1000) as usize)
                    && (device.target.is_some() || selected.contains(&device.address))
                    && let Err(error) = state.toggle_device(device.address)
                {
                    state.log(&format!("SETTINGS ERROR: {error}"));
                    let _ = MessageBoxW(
                        Some(hwnd),
                        w!(
                            "Could not save device preferences. Please check that your user settings folder is writable."
                        ),
                        w!("Stereo Nudge"),
                        MB_OK | MB_ICONERROR,
                    );
                }
            }
            _ => {}
        }
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // Notification buffers are owned by Windows and validated before their fields are read.
    unsafe {
        if let Some(state) = STATE.get() {
            match message {
                WM_DEVICECHANGE if wparam.0 == DBT_CUSTOMEVENT as usize && lparam.0 != 0 => {
                    let pointer = lparam.0 as *const u8;
                    let length = pointer.cast::<u32>().read_unaligned() as usize;
                    if (12..=65536).contains(&length)
                        && let Some((address, kind, connected)) =
                            parse_hci(std::slice::from_raw_parts(pointer, length))
                        && kind == 1
                    {
                        state.on_link(address, connected);
                    }
                }
                WM_DEVICECHANGE if wparam.0 == DBT_DEVNODES_CHANGED as usize => {
                    ensure_radio(state, hwnd);
                }
                WM_DEVICECHANGE
                    if wparam.0 == DBT_DEVICEREMOVECOMPLETE as usize && lparam.0 != 0 =>
                {
                    let pointer = lparam.0 as *const u8;
                    let size = pointer.cast::<u32>().read_unaligned() as usize;
                    if size >= offset_of!(DEV_BROADCAST_HANDLE, dbch_handle) + size_of::<usize>()
                        && pointer.add(4).cast::<u32>().read_unaligned() == DBT_DEVTYP_HANDLE.0
                    {
                        let handle = pointer
                            .add(offset_of!(DEV_BROADCAST_HANDLE, dbch_handle))
                            .cast::<usize>()
                            .read_unaligned();
                        if let Ok(mut binding) = state.radio.lock()
                            && binding
                                .as_ref()
                                .is_some_and(|binding| binding.radio == handle)
                        {
                            binding.take();
                            if let Ok(mut links) = state.links.lock() {
                                links.clear();
                            }
                        }
                    }
                }
                TRAY_MESSAGE
                    if lparam.0 as u32 == WM_RBUTTONUP || lparam.0 as u32 == WM_LBUTTONUP =>
                {
                    show_menu(state, hwnd);
                }
                WM_TIMER => {
                    if state.stop_path.exists() {
                        let _ = DestroyWindow(hwnd);
                    } else if state.started.elapsed().as_secs() < state.startup_wait_seconds {
                        ensure_radio(state, hwnd);
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
        let mutex = CreateMutexW(None, false, w!("Local\\StereoNudge"))?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let _ = CloseHandle(mutex);
            return Err(Error::new(E_FAIL, "Stereo Nudge is already running"));
        }
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
        let tip: Vec<u16> = "Stereo Nudge\0".encode_utf16().collect();
        icon.szTip[..tip.len()].copy_from_slice(&tip);
        if !Shell_NotifyIconW(NIM_ADD, &icon).as_bool() {
            return Err(Error::from_thread());
        }
        ensure_radio(state, hwnd);
        SetTimer(Some(hwnd), 1, 1000, None);
        if state.seconds > 0 {
            let window = hwnd.0 as usize;
            let seconds = state.seconds;
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(seconds));
                let _ = PostMessageW(
                    Some(HWND(window as *mut c_void)),
                    WM_CLOSE,
                    WPARAM(0),
                    LPARAM(0),
                );
            });
        }
        state.log(&format!(
            "ARMED: tray device selection; seconds={}; stop_flag={}",
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
        if let Ok(mut binding) = state.radio.lock() {
            binding.take();
        }
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
        let size = bytes.len() as u32;
        bytes[..4].copy_from_slice(&size.to_le_bytes());
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
        bytes[offset + 9] = 32;
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
