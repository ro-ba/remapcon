//! UpdateProtocol.h / Updater.cpp validation (MIT, copyright 2026 ro-ba).
//! Preserve ../../LICENSE and THIRD_PARTY_NOTICES.md.
#![forbid(unsafe_code)]
pub const MAX_RELEASE_JSON: usize = 1024 * 1024;
pub const MAX_PACKAGE: usize = 100 * 1024 * 1024;
pub const ASSET: &str = "padmux-windows-x64.zip";
#[derive(Clone, Copy)]
#[repr(usize)]
pub enum Failure {
    Network = 1,
    Release = 2,
    Download = 3,
    Checksum = 4,
    Extract = 5,
    Restart = 6,
}
pub fn valid_tag(tag: &str) -> bool {
    if !(6..=32).contains(&tag.len()) || !tag.starts_with('v') {
        return false;
    }
    let mut dots = 0;
    let mut digit = false;
    for ch in tag.bytes().skip(1) {
        if ch == b'.' {
            dots += 1;
            if !digit || dots > 2 {
                return false;
            }
            digit = false;
        } else if ch.is_ascii_digit() {
            digit = true;
        } else {
            return false;
        }
    }
    dots == 2 && digit
}
pub fn release_digest(bytes: &[u8], expected_tag: &str) -> Option<String> {
    let root = crate::config::parse_json(bytes).ok()?;
    if root.get("tag_name")?.as_str()? != expected_tag {
        return None;
    }
    for asset in root.get("assets")?.as_array()? {
        if asset.get("name").and_then(|n| n.as_str()) != Some(ASSET) {
            continue;
        }
        let hash = asset.get("digest")?.as_str()?.strip_prefix("sha256:")?;
        if hash.len() != 64 || !hash.bytes().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        return Some(hash.to_ascii_lowercase());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tags_and_release_digests_match_unchanged_cpp() {
        let reference = std::env::var_os("PADMUX_CONFIG_REFERENCE");
        for (tag, valid) in [
            ("v1.0.0", true),
            ("v0001.02.003", true),
            ("v1.3.10", true),
            ("v1.0", false),
            ("v1.0.0.", false),
            ("v.1.0.0", false),
            ("v1..0", false),
            ("V1.0.0", false),
            ("v1.0.0-beta", false),
            ("v1.0.😀", false),
            ("", false),
            ("v11111111111111111111111111111.0.0", false),
        ] {
            assert_eq!(valid_tag(tag), valid, "{tag}");
            if let Some(exe) = &reference {
                let status = std::process::Command::new(exe)
                    .args(["--validate-tag", tag])
                    .status()
                    .unwrap();
                assert!([Some(0), Some(1)].contains(&status.code()));
                assert_eq!(status.success(), valid, "C++ tag {tag}");
            }
        }
        let hash = "ABCDEF01".repeat(8);
        let valid = serde_json::json!({"tag_name":"v1.0.0","assets":[{"name":ASSET,"digest":format!("sha256:{hash}")}]});
        let mut cases = vec![(
            serde_json::to_vec(&valid).unwrap(),
            Some(hash.to_ascii_lowercase()),
        )];
        for (field, value) in [
            ("tag_name", serde_json::json!("v1.0.1")),
            ("assets", serde_json::json!([])),
        ] {
            let mut json = valid.clone();
            json[field] = value;
            cases.push((serde_json::to_vec(&json).unwrap(), None));
        }
        for digest in [
            serde_json::json!(null),
            serde_json::json!("sha256:abc"),
            serde_json::json!(format!("SHA256:{hash}")),
            serde_json::json!(format!("sha256:{}", "g".repeat(64))),
        ] {
            let mut json = valid.clone();
            json["assets"][0]["digest"] = digest;
            cases.push((serde_json::to_vec(&json).unwrap(), None));
        }
        // First matching asset with a malformed digest fails; do not skip it
        // in favor of a later valid duplicate, matching the reference loop.
        let mut duplicate = valid.clone();
        duplicate["assets"]
            .as_array_mut()
            .unwrap()
            .insert(0, serde_json::json!({"name":ASSET}));
        cases.push((serde_json::to_vec(&duplicate).unwrap(), None));
        for bytes in [
            b"".as_slice(),
            b"bad JSON",
            b"{\"tag_name\":\"v1.0.0\",\"tag_name\":\"v1.0.0\",\"assets\":[]}",
            b"{\"number\":1.5}",
            b"\xef\xbb\xbf{}",
            b"\xff",
        ] {
            cases.push((bytes.to_vec(), None));
        }
        let directory = std::env::temp_dir().join(format!(
            "padmux-update-reference-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        for (bytes, expected) in cases {
            assert_eq!(release_digest(&bytes, "v1.0.0"), expected);
            if let Some(exe) = &reference {
                let input = directory.join("release.json");
                let output = directory.join("digest.json");
                std::fs::write(&input, bytes).unwrap();
                assert!(std::process::Command::new(exe)
                    .arg("--release-digest")
                    .arg(input)
                    .arg("v1.0.0")
                    .arg(&output)
                    .status()
                    .unwrap()
                    .success());
                assert_eq!(
                    serde_json::from_slice::<Option<String>>(&std::fs::read(output).unwrap())
                        .unwrap(),
                    expected
                );
            }
        }
        std::fs::remove_dir_all(directory).unwrap();
    }
}
