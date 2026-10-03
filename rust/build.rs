use std::{env, fs, path::PathBuf, process::Command};
fn main() {
    println!("cargo:rerun-if-changed=../src/app/resources");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows")
        || env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc")
    {
        return;
    }
    // Reuse the existing licensed ICO/version resource and installed Windows
    // SDK resource compiler; no downloaded resource/build crate is needed.
    let kits = env::var_os("WindowsSdkDir")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env::var_os("ProgramFiles(x86)").expect("ProgramFiles(x86) missing"))
                .join("Windows Kits/10")
        });
    let mut versions: Vec<_> = fs::read_dir(kits.join("bin"))
        .expect("Windows SDK missing")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.join("x64/rc.exe").is_file())
        .collect();
    versions.sort();
    let bin = versions.last().expect("Windows SDK x64 rc.exe missing");
    let version = bin.file_name().unwrap();
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("padmux.res");
    let status = Command::new(bin.join("x64/rc.exe"))
        .current_dir(root.join("../src/app/resources"))
        .arg("/nologo")
        .arg("/fo")
        .arg(&output)
        .arg("/I")
        .arg(kits.join("Include").join(version).join("um"))
        .arg("/I")
        .arg(kits.join("Include").join(version).join("shared"))
        .arg("app.rc")
        .status()
        .expect("could not launch resource compiler");
    assert!(
        status.success(),
        "existing app.rc resource compilation failed"
    );
    println!("cargo:rustc-link-arg-bin=padmux={}", output.display());
}
