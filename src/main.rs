#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod app;
mod audio;
mod config;

use std::{
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

struct State {
    target: audio::Target,
    address: u64,
    enabled: AtomicBool,
    linked: AtomicBool,
    busy: AtomicBool,
    connected_at: Mutex<Option<Instant>>,
    log_path: PathBuf,
    stop_path: PathBuf,
    seconds: u64,
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
    fn request(self: &Arc<Self>, manual: bool) {
        if (!manual && !self.enabled.load(Ordering::Acquire))
            || self.busy.swap(true, Ordering::AcqRel)
        {
            return;
        }
        let state = self.clone();
        std::thread::spawn(move || {
            let result = (|| -> Result<()> {
                let _com = audio::ComApartment::new(COINIT_MULTITHREADED)?;
                state.log("REQUEST START: stereo reconnect");
                let start = Instant::now();
                audio::request(&state.target)?;
                state.log(&format!(
                    "REQUEST ACCEPTED: duration_us={}; connection not yet confirmed",
                    start.elapsed().as_micros()
                ));
                Ok(())
            })();
            if let Err(error) = result {
                state.log(&format!("REQUEST ERROR: {error}"));
            }
            state.busy.store(false, Ordering::Release);
        });
    }
}
fn run() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let options = config::Options::parse(std::env::args().skip(1))?;
    let _com = audio::ComApartment::new(COINIT_APARTMENTTHREADED)?;
    fs::create_dir_all(&options.log_dir)?;
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let target = audio::find_stereo(options.address)?;
    let state = Arc::new(State {
        target,
        address: options.address,
        enabled: AtomicBool::new(true),
        linked: AtomicBool::new(false),
        busy: AtomicBool::new(false),
        connected_at: Mutex::new(None),
        log_path: options.log_dir.join(format!("rust-{stamp}.log")),
        stop_path: options
            .log_dir
            .join(format!("stop-{}.flag", std::process::id())),
        seconds: options.seconds,
    });
    state.log(&format!(
        "START: native Rust; pid={}; stereo endpoint={}",
        std::process::id(),
        state.target.endpoint
    ));
    audio::probe(&state.target)?;
    state.log("PROBE: A2DP driver supports reconnect");
    if options.probe {
        return Ok(());
    }
    if options.request {
        audio::request(&state.target)?;
        state.log("REQUEST ACCEPTED");
        return Ok(());
    }
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
