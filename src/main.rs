#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
mod audio;
mod config;
mod devices;
mod settings;

use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use windows::{Win32::System::Com::*, core::Result};
static STATE: OnceLock<Arc<State>> = OnceLock::new();

struct Link {
    connected: bool,
    since: Option<Instant>,
}
struct State {
    devices: Mutex<Vec<devices::PairedDevice>>,
    selected: Mutex<BTreeSet<u64>>,
    links: Mutex<HashMap<u64, Link>>,
    busy: Mutex<HashSet<u64>>,
    enabled: AtomicBool,
    settings_path: PathBuf,
    log_path: PathBuf,
    stop_path: PathBuf,
    seconds: u64,
    startup_wait_seconds: u64,
    started: Instant,
    radio: Mutex<Option<app::RadioBinding>>,
}
impl State {
    fn log(&self, message: &str) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let line = format!("unix_ms={timestamp} {message}\n");
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
        {
            let _ = file.write_all(line.as_bytes());
        }
        print!("{line}");
    }
    fn refresh_devices(&self) {
        match devices::paired_devices() {
            Ok(devices) => {
                if let Ok(mut current) = self.devices.lock() {
                    *current = devices;
                }
            }
            Err(error) => self.log(&format!("DEVICE LIST: {error}")),
        }
    }
    fn toggle_device(
        self: &Arc<Self>,
        address: u64,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut selected = self.selected.lock().map_err(|_| "Settings lock poisoned")?;
        let mut updated = selected.clone();
        let enable = !updated.remove(&address);
        if enable {
            updated.insert(address);
        }
        settings::save(&self.settings_path, &updated)?;
        *selected = updated;
        drop(selected);
        if let Ok(mut links) = self.links.lock() {
            links.remove(&address);
        }
        self.log(&format!(
            "DEVICE SELECTED: address={address:012X}; enabled={enable}"
        ));
        if enable {
            self.request(address, false);
        }
        Ok(())
    }
    fn on_link(self: &Arc<Self>, address: u64, connected: bool) {
        let selected = self
            .selected
            .lock()
            .map(|set| set.contains(&address))
            .unwrap_or(false);
        if !selected {
            return;
        }
        let mut rising = false;
        if let Ok(mut links) = self.links.lock() {
            let link = links.entry(address).or_insert(Link {
                connected: false,
                since: None,
            });
            rising = connected && !link.connected;
            link.connected = connected;
            if rising {
                link.since = Some(Instant::now());
            }
        }
        self.log(&format!(
            "HCI ACL: address={address:012X}; connected={connected}"
        ));
        if rising {
            self.request(address, false);
        }
    }
    fn request(self: &Arc<Self>, address: u64, manual: bool) {
        if !manual && !self.enabled.load(Ordering::Acquire) {
            return;
        }
        let Ok(mut busy) = self.busy.lock() else {
            return;
        };
        if !busy.insert(address) {
            return;
        }
        drop(busy);
        let state = self.clone();
        std::thread::spawn(move || {
            let result = (|| -> Result<()> {
                let _com = audio::ComApartment::new(COINIT_MULTITHREADED)?;
                state.log(&format!("REQUEST START: address={address:012X}"));
                let target = audio::find_stereo(address)?;
                if let Ok(mut devices) = state.devices.lock()
                    && let Some(device) =
                        devices.iter_mut().find(|device| device.address == address)
                {
                    device.target = Some(target.clone());
                }
                let start = Instant::now();
                audio::request(&target)?;
                state.log(&format!("REQUEST ACCEPTED: address={address:012X}; duration_us={}; connection not yet confirmed", start.elapsed().as_micros()));
                Ok(())
            })();
            if let Err(error) = result {
                state.log(&format!("REQUEST ERROR: address={address:012X}; {error}"));
            }
            if let Ok(mut busy) = state.busy.lock() {
                busy.remove(&address);
            }
        });
    }
    fn endpoint_changed(&self, id: &str, endpoint_state: u32) {
        let address = self.devices.lock().ok().and_then(|devices| {
            devices
                .iter()
                .find(|device| {
                    device
                        .target
                        .as_ref()
                        .is_some_and(|target| target.endpoint.eq_ignore_ascii_case(id))
                })
                .map(|device| device.address)
        });
        if let Some(address) = address {
            let elapsed = self.links.lock().ok().and_then(|links| {
                links
                    .get(&address)
                    .and_then(|link| link.since.map(|time| time.elapsed().as_secs_f64() * 1000.0))
            });
            self.log(&format!("STEREO STATE={endpoint_state}; address={address:012X}; since_connection_ms={elapsed:?}"));
        }
    }
}

fn run() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let options = config::Options::parse(std::env::args().skip(1))?;
    let _com = audio::ComApartment::new(COINIT_APARTMENTTHREADED)?;
    if let Some(path) = options.list_devices_path {
        devices::write_paired_headphones(&path)?;
        return Ok(());
    }
    fs::create_dir_all(&options.log_dir)?;
    if options.probe || options.request {
        let target = devices::wait_for_target(options.address, options.startup_wait_seconds)?;
        audio::probe(&target)?;
        if options.request {
            audio::request(&target)?;
        }
        return Ok(());
    }
    let mut selected = settings::load(&options.settings_path)?;
    if options.address != 0 {
        selected.insert(options.address);
        settings::save(&options.settings_path, &selected)?;
    }
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let state = Arc::new(State {
        devices: Mutex::new(Vec::new()),
        selected: Mutex::new(selected),
        links: Mutex::new(HashMap::new()),
        busy: Mutex::new(HashSet::new()),
        enabled: AtomicBool::new(true),
        settings_path: options.settings_path,
        log_path: options.log_dir.join(format!("rust-{stamp}.log")),
        stop_path: options
            .log_dir
            .join(format!("stop-{}.flag", std::process::id())),
        seconds: options.seconds,
        startup_wait_seconds: options.startup_wait_seconds,
        started: Instant::now(),
        radio: Mutex::new(None),
    });
    state.log(&format!(
        "START: native Rust; pid={}; settings={}",
        std::process::id(),
        state.settings_path.display()
    ));
    state.refresh_devices();
    STATE
        .set(state.clone())
        .map_err(|_| "State initialized twice")?;
    app::run(&state)?;
    Ok(())
}
fn main() -> std::process::ExitCode {
    if let Err(error) = run() {
        eprintln!("Stereo Nudge: {error}");
        let _ = fs::write(
            std::env::temp_dir().join("stereo-nudge-error.txt"),
            error.to_string(),
        );
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tray_choices_persist_and_reselection_resets_link_tracking() {
        let directory =
            std::env::temp_dir().join(format!("stereo-nudge-tray-test-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let state = Arc::new(State {
            devices: Mutex::new(Vec::new()),
            selected: Mutex::new(BTreeSet::new()),
            links: Mutex::new(HashMap::new()),
            busy: Mutex::new(HashSet::new()),
            enabled: AtomicBool::new(false),
            settings_path: directory.join("devices.txt"),
            log_path: directory.join("app.log"),
            stop_path: directory.join("stop.flag"),
            seconds: 0,
            startup_wait_seconds: 0,
            started: Instant::now(),
            radio: Mutex::new(None),
        });
        let address = 0xaabbccddeeff;
        state.toggle_device(address).unwrap();
        assert!(
            settings::load(&state.settings_path)
                .unwrap()
                .contains(&address)
        );
        state.on_link(address, true);
        assert!(state.links.lock().unwrap()[&address].connected);
        state.toggle_device(address).unwrap();
        assert!(settings::load(&state.settings_path).unwrap().is_empty());
        assert!(!state.links.lock().unwrap().contains_key(&address));
        state.toggle_device(address).unwrap();
        state.on_link(address, true);
        assert!(state.links.lock().unwrap()[&address].since.is_some());
        assert!(state.busy.lock().unwrap().is_empty()); // Globally paused: no driver request.
        fs::remove_file(&state.settings_path).unwrap();
        fs::remove_file(&state.log_path).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}
