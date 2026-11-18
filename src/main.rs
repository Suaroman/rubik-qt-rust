//! Rubik's Cube DNA observatory — Qt edition.
//!
//! The Rust engine (cube state, timeline, net permutation, lighting model) is
//! exposed to QML through Qt Bridge for Rust as a single `Rubik` singleton. QML
//! drives the clock: a `FrameAnimation` calls `Rubik.tick(dt)` once per frame and
//! gets back a plain JS object describing everything on screen. Qt Quick 3D
//! renders the cube; Qt Quick draws the HUD and the two instrument rails.
//!
//! All QML, fonts and the icon are compiled into the binary as Qt resources, so
//! the executable only needs the Qt libraries at run time.

// A GUI program: on Windows a release build is a GUI-subsystem executable, so
// double-clicking it opens only the cube window and no console.
// `attach_parent_console` below keeps terminal output working when it is
// started from PowerShell or cmd (as the softbuffer version did).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use qtbridge::{QApp, include_bytes_qml, qobject};
use serde_json::{Value, json};

use rubik_qt::sim::{Sim, static_info};

#[derive(Default)]
pub struct Rubik {
    sim: Sim,
}

#[qobject(Singleton)]
impl Rubik {
    /// Advance by `dt` seconds of wall time and return the frame snapshot.
    #[qslot]
    fn tick(&mut self, dt: f64) -> Value {
        self.sim.step(dt);
        self.sim.snapshot()
    }

    /// The current snapshot without advancing time.
    #[qslot]
    fn snapshot(&self) -> Value {
        self.sim.snapshot()
    }

    /// Headless pre-roll: step at 60 Hz to `at` seconds (for `--shot`).
    #[qslot]
    fn prestep(&mut self, at: f64) -> Value {
        self.sim.prestep(at);
        self.sim.snapshot()
    }

    /// Net layout, palette and the sticker table: things that never change.
    #[qslot(qml_name = "staticInfo")]
    fn static_info(&self) -> Value {
        static_info()
    }

    /// Rounded cubie body and bevelled sticker geometry.
    #[qslot]
    fn meshes(&self) -> Value {
        rubik_qt::mesh::meshes_json()
    }

    #[qslot(qml_name = "toggleOrbit")]
    fn toggle_orbit(&mut self) {
        let l = &mut self.sim.eng.light;
        l.orbit = !l.orbit;
    }

    #[qslot(qml_name = "togglePause")]
    fn toggle_pause(&mut self) {
        self.sim.paused = !self.sim.paused;
    }

    #[qslot(qml_name = "setSpeed")]
    fn set_speed(&mut self, s: f64) {
        self.sim.speed = s.clamp(0.05, 8.0);
    }

    #[qslot(qml_name = "setHover")]
    fn set_hover(&mut self, idx: i32) {
        self.sim.hover = if (0..54).contains(&idx) { idx } else { -1 };
    }

    /// Hover from the 3D view (ray pick): cubie index + sticker slot, -1 for plastic.
    #[qslot(qml_name = "hoverSticker")]
    fn hover_sticker(&mut self, cubie: i32, slot: i32) -> i32 {
        if !(0..26).contains(&cubie) {
            self.sim.hover = -1;
            return -1;
        }
        self.sim.hover_sticker(cubie, slot)
    }

    #[qslot(qml_name = "orbitBy")]
    fn orbit_by(&mut self, dx: f64, dy: f64) {
        self.sim.orbit_by(dx, dy);
    }

    #[qslot(qml_name = "releaseOrbit")]
    fn release_orbit(&mut self, vx: f64, vy: f64) {
        self.sim.release(vx, vy);
    }

    #[qslot(qml_name = "resetView")]
    fn reset_view(&mut self) {
        self.sim.reset_view();
    }

    /// Prints a line to stdout (QML's console goes to stderr with a prefix).
    #[qslot]
    fn log(&self, msg: String) {
        rubik_qt::log_line(&msg);
    }

    /// Command-line options QML cares about.
    #[qslot(qml_name = "launchOptions")]
    fn launch_options(&self) -> Value {
        let args: Vec<String> = std::env::args().collect();
        let get = |flag: &str| {
            args.iter()
                .position(|a| a == flag)
                .and_then(|p| args.get(p + 1))
                .cloned()
        };
        let num = |flag: &str, d: f64| get(flag).and_then(|s| s.parse::<f64>().ok()).unwrap_or(d);
        json!({
            "shot": get("--shot").unwrap_or_default(),
            "at": num("--at", 12.0),
            "width": num("--width", 1440.0),
            "height": num("--height", 900.0),
            "quality": get("--quality").unwrap_or_else(|| "high".into()),
            "fullscreen": args.iter().any(|a| a == "--fullscreen"),
            // capture helpers for --shot
            "explode": args.iter().any(|a| a == "--explode"),
            "celebrate": args.iter().any(|a| a == "--celebrate"),
            "hover": num("--hover", -1.0),
            "delay": num("--delay", 1200.0),
            "quitAfter": num("--quit-after", 0.0),
            "only3d": args.iter().any(|a| a == "--only-3d"),
            "screen": num("--screen", -1.0),
            "pointX": num("--pointer-x", -1.0),
            "pointY": num("--pointer-y", -1.0),
        })
    }
}

/// Under WSL, Mesa's default (Zink) fails and Qt silently falls back to CPU
/// rendering — about 3 fps for this scene. The d3d12 Gallium driver reaches the
/// real GPU, and presenting through XWayland is markedly faster than WSLg's
/// Wayland path. Only applied when the user has not chosen either themselves.
fn prefer_wsl_gpu() {
    let wsl = std::env::var_os("WSL_DISTRO_NAME").is_some()
        || std::path::Path::new("/proc/sys/fs/binfmt_misc/WSLInterop").exists();
    if !wsl || std::env::var_os("RUBIK_NO_WSL_TWEAKS").is_some() {
        return;
    }
    let has_d3d12 = std::path::Path::new("/usr/lib/x86_64-linux-gnu/dri/d3d12_dri.so").exists();
    // SAFETY: called first thing in main, before Qt or any other thread starts.
    unsafe {
        if has_d3d12 && std::env::var_os("GALLIUM_DRIVER").is_none() {
            std::env::set_var("GALLIUM_DRIVER", "d3d12");
        }
        if std::env::var_os("QT_QPA_PLATFORM").is_none() && std::env::var_os("DISPLAY").is_some() {
            std::env::set_var("QT_QPA_PLATFORM", "xcb");
        }
    }
}

/// Reconnect stdout/stderr to the terminal that started us (Windows only).
#[cfg(windows)]
fn attach_parent_console() {
    const ATTACH_PARENT_PROCESS: u32 = 0xFFFF_FFFF;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn AttachConsole(attachment: u32) -> i32;
    }
    unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
}

#[cfg(not(windows))]
fn attach_parent_console() {}

/// On hybrid-graphics laptops Qt's D3D11 backend takes DXGI adapter 0, which
/// is usually the integrated GPU (measured: Intel UHD instead of an RTX A5000).
/// Pick the hardware adapter with the most dedicated VRAM instead, unless the
/// user has chosen one with `QT_D3D_ADAPTER_INDEX`.
#[cfg(windows)]
fn prefer_discrete_gpu() {
    use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE};
    if std::env::var_os("QT_D3D_ADAPTER_INDEX").is_some() {
        return;
    }
    let Ok(factory) = (unsafe { CreateDXGIFactory1::<IDXGIFactory1>() }) else { return };
    let mut best: Option<(u32, usize, String)> = None;
    let mut i = 0u32;
    while let Ok(adapter) = unsafe { factory.EnumAdapters1(i) } {
        if let Ok(d) = unsafe { adapter.GetDesc1() } {
            let software = d.Flags & (DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32) != 0;
            if !software && best.as_ref().is_none_or(|b| d.DedicatedVideoMemory > b.1) {
                let n = d.Description.iter().position(|c| *c == 0).unwrap_or(d.Description.len());
                best = Some((i, d.DedicatedVideoMemory, String::from_utf16_lossy(&d.Description[..n])));
            }
        }
        i += 1;
    }
    if let Some((idx, mem, name)) = best {
        rubik_qt::log_line(&format!("[gpu] adapter {idx}: {name} ({} MB VRAM)", mem / (1 << 20)));
        set_env_for_qt("QT_D3D_ADAPTER_INDEX", &idx.to_string());
    }
}

/// Sets a variable where Qt will see it. On Windows Qt reads the C runtime's
/// copy of the environment (`getenv_s`), which `SetEnvironmentVariableW` (what
/// `std::env::set_var` calls) does not update, so the CRT copy is set too.
#[cfg(windows)]
fn set_env_for_qt(key: &str, value: &str) {
    use std::ffi::{c_char, CString};
    unsafe extern "C" {
        fn _putenv_s(name: *const c_char, value: *const c_char) -> i32;
    }
    let (k, v) = (CString::new(key).unwrap(), CString::new(value).unwrap());
    // SAFETY: called first thing in main, before Qt or any other thread starts.
    unsafe {
        std::env::set_var(key, value);
        _putenv_s(k.as_ptr(), v.as_ptr());
    }
}

#[cfg(not(windows))]
fn prefer_discrete_gpu() {}

fn main() {
    attach_parent_console();
    prefer_wsl_gpu();
    prefer_discrete_gpu();
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        rubik_qt::log_line(&format!(
            "rubik_qt — Rubik's Cube DNA observatory (Qt edition)\n\n\
             Options:\n  \
             --shot <file.png>   render one frame at --at seconds, save it, quit\n  \
             --at <secs>         clock reading for --shot (default 12)\n  \
             --width/--height    window size in logical px (default 1440x900)\n  \
             --quality <q>       high | low  (post-processing; low for software GL)\n  \
             --fullscreen        start fullscreen\n  \
             --quit-after <secs> run live, print fps once a second, then quit\n  \
             --screen <n>        open on monitor n (0-based; listed with --quit-after)\n  \
             --explode, --hover <cell 0-53>, --celebrate,\n  \
             --pointer-x/--pointer-y <px>   simulated mouse position  (with --shot)\n"
        ));
        return;
    }
    include_bytes_qml!("qml", "app");
    QApp::new()
        .application_name("Rubik's Cube — Qt")
        .register::<Rubik>()
        .load_qml_from_file("qrc:/app/qml/Main.qml")
        .run();
}
