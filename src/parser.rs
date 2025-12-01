use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

#[derive(Debug, Default)]
pub struct PackageMetaData {
    pub pkgbase: String,
    pub version: String,
    pub depends: Vec<String>,
    pub make_depends: Vec<String>,
    pub validpgpkeys: Vec<String>,
}

pub fn parse_srcinfo(path: &Path) -> io::Result<PackageMetaData> {
    let file = File::open(path.join(".SRCINFO"))?;
    let reader = io::BufReader::new(file);

    let mut metadata = PackageMetaData::default();

    for line in reader.lines() {
        let line = line?; // unwrap the Result<String>
        let line = line.trim();

        // Skip empty lines or comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // The format is "key = value"
        // We split once. If there is no '=', it's a weird line, skip it.
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim().to_string();

            match key {
                "pkgbase" => metadata.pkgbase = value,
                "pkgver" => metadata.version = value,
                "depends" => metadata.depends.push(value),
                "makedepends" => metadata.make_depends.push(value),
                "validpgpkeys" => metadata.validpgpkeys.push(value),
                // We ignore keys we don't care about (like 'arch', 'license')
                _ => {}
            }
        }
    }

    Ok(metadata)
}