//! Auto-start management for Windows
//!
//! Enables/disables CodexBar to start automatically when Windows boots

use clap::Args;
use std::path::PathBuf;

#[cfg(target_os = "windows")]
use std::ffi::OsStr;
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;

const APP_NAME: &str = "CodexBar";

#[derive(Args, Debug)]
pub struct AutostartArgs {
    /// Enable auto-start on Windows boot
    #[arg(long, conflicts_with = "disable")]
    pub enable: bool,

    /// Disable auto-start
    #[arg(long, conflicts_with = "enable")]
    pub disable: bool,

    /// Show current auto-start status
    #[arg(long, conflicts_with_all = ["enable", "disable"])]
    pub status: bool,
}

pub async fn run(args: AutostartArgs) -> anyhow::Result<()> {
    if args.enable {
        enable_autostart()?;
        println!("Auto-start enabled. CodexBar will start when Windows boots.");
    } else if args.disable {
        disable_autostart()?;
        println!("Auto-start disabled.");
    } else {
        // Default: show status
        let enabled = is_autostart_enabled();
        if enabled {
            println!("Auto-start is enabled.");
        } else {
            println!("Auto-start is disabled.");
        }
    }
    Ok(())
}

/// Get the path to the current executable
fn get_exe_path() -> anyhow::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    Ok(exe)
}

/// Enable auto-start by adding registry entry
#[cfg(target_os = "windows")]
fn enable_autostart() -> anyhow::Result<()> {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{
        RegSetValueExW, RegOpenKeyExW, RegCloseKey, HKEY_CURRENT_USER, KEY_WRITE, REG_SZ,
    };

    let exe_path = get_exe_path()?;
    let command = format!("\"{}\" menubar", exe_path.display());

    let subkey = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
    let subkey_wide: Vec<u16> = OsStr::new(subkey)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let name_wide: Vec<u16> = OsStr::new(APP_NAME)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let value_wide: Vec<u16> = OsStr::new(&command)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let mut hkey = windows::Win32::System::Registry::HKEY::default();

        let result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey_wide.as_ptr()),
            0,
            KEY_WRITE,
            &mut hkey,
        );

        if result.is_err() {
            return Err(anyhow::anyhow!("Failed to open registry key: {:?}", result));
        }

        let result = RegSetValueExW(
            hkey,
            PCWSTR(name_wide.as_ptr()),
            0,
            REG_SZ,
            Some(&value_wide.align_to::<u8>().1[..value_wide.len() * 2]),
        );

        let _ = RegCloseKey(hkey);

        if result.is_err() {
            return Err(anyhow::anyhow!("Failed to set registry value: {:?}", result));
        }
    }

    Ok(())
}

/// Disable auto-start by removing registry entry
#[cfg(target_os = "windows")]
fn disable_autostart() -> anyhow::Result<()> {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{
        RegDeleteValueW, RegOpenKeyExW, RegCloseKey, HKEY_CURRENT_USER, KEY_WRITE,
    };

    let subkey = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
    let subkey_wide: Vec<u16> = OsStr::new(subkey)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let name_wide: Vec<u16> = OsStr::new(APP_NAME)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let mut hkey = windows::Win32::System::Registry::HKEY::default();

        let result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey_wide.as_ptr()),
            0,
            KEY_WRITE,
            &mut hkey,
        );

        if result.is_err() {
            // Key doesn't exist, already disabled
            return Ok(());
        }

        let _ = RegDeleteValueW(hkey, PCWSTR(name_wide.as_ptr()));
        let _ = RegCloseKey(hkey);
    }

    Ok(())
}

/// Check if auto-start is enabled
#[cfg(target_os = "windows")]
fn is_autostart_enabled() -> bool {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{
        RegQueryValueExW, RegOpenKeyExW, RegCloseKey, HKEY_CURRENT_USER, KEY_READ,
    };

    let subkey = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
    let subkey_wide: Vec<u16> = OsStr::new(subkey)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let name_wide: Vec<u16> = OsStr::new(APP_NAME)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let mut hkey = windows::Win32::System::Registry::HKEY::default();

        let result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey_wide.as_ptr()),
            0,
            KEY_READ,
            &mut hkey,
        );

        if result.is_err() {
            return false;
        }

        let result = RegQueryValueExW(
            hkey,
            PCWSTR(name_wide.as_ptr()),
            None,
            None,
            None,
            None,
        );

        let _ = RegCloseKey(hkey);

        result.is_ok()
    }
}

#[cfg(not(target_os = "windows"))]
fn enable_autostart() -> anyhow::Result<()> {
    Err(anyhow::anyhow!("Auto-start is only supported on Windows"))
}

#[cfg(not(target_os = "windows"))]
fn disable_autostart() -> anyhow::Result<()> {
    Err(anyhow::anyhow!("Auto-start is only supported on Windows"))
}

#[cfg(not(target_os = "windows"))]
fn is_autostart_enabled() -> bool {
    false
}
