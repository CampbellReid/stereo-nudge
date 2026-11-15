use std::path::PathBuf;
pub struct Options {
    pub address: u64,
    pub seconds: u64,
    pub log_dir: PathBuf,
    pub probe: bool,
    pub request: bool,
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
        let mut result = Self {
            address: 0,
            seconds: 600,
            log_dir: PathBuf::from("logs"),
            probe: false,
            request: false,
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
                "--probe" => result.probe = true,
                "--request" => result.request = true,
                "--help" | "-h" => {
                    println!(
                        "stereo-nudge --address AA:BB:CC:DD:EE:FF [--seconds 600] [--log-dir logs] [--probe | --request]\nDuration 0 runs until Exit. No startup registration or profile changes."
                    );
                    std::process::exit(0);
                }
                _ => return Err(format!("Unknown argument: {arg}")),
            }
        }
        if result.address == 0 {
            return Err("Supply --address with the paired headset's Bluetooth address".into());
        }
        if result.probe && result.request {
            return Err("Choose --probe or --request, not both".into());
        }
        Ok(result)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
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
}
