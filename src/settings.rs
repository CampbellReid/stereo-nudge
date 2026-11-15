use crate::config::parse_address;
use std::{collections::BTreeSet, fs, path::Path};
pub fn load(path: &Path) -> std::io::Result<BTreeSet<u64>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text
            .lines()
            .filter_map(|line| parse_address(line.trim()).ok())
            .filter(|address| *address != 0)
            .collect()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(BTreeSet::new()),
        Err(error) => Err(error),
    }
}
pub fn save(path: &Path, addresses: &BTreeSet<u64>) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text: String = addresses
        .iter()
        .map(|address| format!("{address:012X}\n"))
        .collect();
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, text)?;
    fs::rename(temporary, path)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preferences_survive_replacement_and_ignore_corrupt_lines() {
        let directory =
            std::env::temp_dir().join(format!("stereo-nudge-settings-{}", std::process::id()));
        let path = directory.join("devices.txt");
        save(&path, &BTreeSet::from([0xaabbccddeeff])).unwrap();
        save(&path, &BTreeSet::from([0x112233445566])).unwrap();
        assert_eq!(load(&path).unwrap(), BTreeSet::from([0x112233445566]));
        fs::write(&path, "garbage\n112233445566\n000000000000\n").unwrap();
        assert_eq!(load(&path).unwrap(), BTreeSet::from([0x112233445566]));
        fs::remove_file(path).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}
