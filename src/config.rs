use std::path::PathBuf;
pub struct Options {
    pub address: u64,
    pub seconds: u64,
    pub log_dir: PathBuf,
    pub probe: bool,
    pub request: bool,
    pub list_devices_path: Option<PathBuf>,
    pub startup_wait_seconds: u64,
    pub settings_path: PathBuf,
}
pub fn parse_address(value: &str) -> Result<u64, String> {
    let compact = value.replace([':', '-'], "");
    if compact.len() != 12 || !compact.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(
            "Address must contain exactly 12 hexadecimal digits, optionally separated by : or -"
                .into(),
        );
    }
    u64::from_str_radix(&compact, 16).map_err(|error| error.to_string())
}
impl Options {
    pub fn parse(args: impl Iterator<Item = String>) -> Result<Self, String> {
        let data_dir =
            PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_else(|| ".".into()))
                .join("StereoNudge");
        let mut result = Self {
            address: 0,
            seconds: 0,
            log_dir: data_dir.join("logs"),
            probe: false,
            request: false,
            list_devices_path: None,
            startup_wait_seconds: 60,
            settings_path: data_dir.join("devices.txt"),
        };
        let mut args = args;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--address" => {
                    result.address = parse_address(&args.next().ok_or("Missing address")?)?
                }
                "--seconds" => {
                    result.seconds = args
                        .next()
                        .ok_or("Missing duration")?
                        .parse()
                        .map_err(|_| "Invalid seconds")?
                }
                "--log-dir" => result.log_dir = args.next().ok_or("Missing log directory")?.into(),
                "--settings-file" => {
                    result.settings_path = args.next().ok_or("Missing settings file")?.into()
                }
                "--list-devices" => {
                    result.list_devices_path =
                        Some(args.next().ok_or("Missing device list path")?.into())
                }
                "--startup-wait-seconds" => {
                    result.startup_wait_seconds = args
                        .next()
                        .ok_or("Missing startup wait")?
                        .parse()
                        .map_err(|_| "Invalid startup wait")?;
                    if result.startup_wait_seconds > 300 {
                        return Err("Startup wait must be at most 300 seconds".into());
                    }
                }
                "--probe" => result.probe = true,
                "--request" => result.request = true,
                "--help" | "-h" => {
                    println!(
                        "stereo-nudge [--seconds 0] [--log-dir logs] [--startup-wait-seconds 60]\nSelect devices from the tray menu; choices are saved automatically.\nOptional: --address AA:BB:CC:DD:EE:FF preselects a device; --probe or --request requires an address.\n--list-devices OUTPUT.tsv lists paired stereo devices."
                    );
                    std::process::exit(0);
                }
                _ => return Err(format!("Unknown argument: {arg}")),
            }
        }
        if result.address == 0 && (result.probe || result.request) {
            return Err("Supply --address for --probe or --request".into());
        }
        if result.probe && result.request {
            return Err("Choose --probe or --request, not both".into());
        }
        if result.list_devices_path.is_some() && (result.probe || result.request) {
            return Err("Device listing cannot be combined with an audio request".into());
        }
        Ok(result)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tray_can_start_without_a_selected_device() {
        assert_eq!(Options::parse(std::iter::empty()).unwrap().address, 0);
    }
    #[test]
    fn parses_address_formats() {
        for value in ["AA:BB:CC:DD:EE:FF", "aa-bb-cc-dd-ee-ff", "aabbccddeeff"] {
            assert_eq!(parse_address(value).unwrap(), 0xaabbccddeeff);
        }
    }
    #[test]
    fn rejects_invalid_addresses() {
        for value in ["", "123", "00112233445566", "GG1122334455", "00112233445 "] {
            assert!(parse_address(value).is_err(), "{value}");
        }
    }
    #[test]
    fn device_listing_does_not_require_an_address() {
        let options = Options::parse(
            ["--list-devices", "devices.tsv"]
                .into_iter()
                .map(str::to_owned),
        )
        .unwrap();
        assert!(options.list_devices_path.is_some());
    }
    #[test]
    fn startup_wait_is_bounded() {
        assert!(
            Options::parse(
                ["--address", "aabbccddeeff", "--startup-wait-seconds", "301"]
                    .into_iter()
                    .map(str::to_owned)
            )
            .is_err()
        );
    }
}
