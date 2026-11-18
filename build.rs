//! Windows: embeds `icon/rubik_qt.rc` (the app icon, as `IDI_ICON1`, which Qt
//! uses for the titlebar/taskbar and Explorer shows for the file).
//!
//! Linux: bakes the Qt library directory into the executable's RUNPATH so it
//! starts without `LD_LIBRARY_PATH`. Qt is found the same way Qt Bridge finds it: the
//! `QMAKE` env var (see `.cargo/config.toml`), else `qmake6` / `qmake` on PATH.

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=QMAKE");
    println!("cargo:rerun-if-changed=icon/rubik_qt.rc");
    println!("cargo:rerun-if-changed=icon/icon.ico");
    #[cfg(windows)]
    embed_resource::compile("icon/rubik_qt.rc", embed_resource::NONE)
        .manifest_optional()
        .expect("embedding the icon resource failed");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("linux") {
        return;
    }
    let candidates: Vec<String> = std::env::var("QMAKE")
        .into_iter()
        .chain(["qmake6".to_string(), "qmake".to_string()])
        .collect();
    for qmake in candidates {
        let Ok(out) = Command::new(&qmake).args(["-query", "QT_INSTALL_LIBS"]).output() else {
            continue;
        };
        if !out.status.success() {
            continue;
        }
        let lib = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !lib.is_empty() {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{lib}");
            return;
        }
    }
    println!("cargo:warning=qmake not found: binary will need LD_LIBRARY_PATH to find Qt");
}
