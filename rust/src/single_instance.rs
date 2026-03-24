//! Single instance detection using Windows named mutex
//!
//! Prevents multiple instances of the menubar app from running simultaneously.

#[cfg(windows)]
use windows::Win32::Foundation::{CloseHandle, HANDLE};
#[cfg(windows)]
use windows::Win32::System::Threading::{CreateMutexW, ReleaseMutex};
#[cfg(windows)]
use windows::core::PCWSTR;

/// Guard that holds the single instance mutex
/// When dropped, the mutex is released
pub struct SingleInstanceGuard {
    #[cfg(windows)]
    handle: HANDLE,
    #[cfg(not(windows))]
    _marker: std::marker::PhantomData<()>,
}

impl SingleInstanceGuard {
    /// Mutex name for CodexBar — uses Local namespace to restrict to current session,
    /// preventing other users/sessions from blocking startup.
    const MUTEX_NAME: &'static str = "Local\\CodexBar_SingleInstance_Mutex";

    /// Try to acquire the single instance lock
    /// Returns Some(guard) if this is the first instance, None if another instance is running
    #[cfg(windows)]
    pub fn try_acquire() -> Option<Self> {
        use windows::Win32::Foundation::GetLastError;
        use windows::Win32::Foundation::ERROR_ALREADY_EXISTS;

        // Convert mutex name to wide string
        let wide_name: Vec<u16> = Self::MUTEX_NAME
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let handle = CreateMutexW(
                None,           // default security attributes
                true,           // initially owned
                PCWSTR(wide_name.as_ptr()),
            );

            match handle {
                Ok(h) => {
                    // Check if mutex already existed
                    let last_error = GetLastError();
                    if last_error == ERROR_ALREADY_EXISTS {
                        // Another instance is running, close our handle
                        let _ = CloseHandle(h);
                        None
                    } else {
                        // We're the first instance
                        Some(Self { handle: h })
                    }
                }
                Err(_) => {
                    // Failed to create mutex, allow running anyway
                    tracing::warn!("Failed to create single instance mutex");
                    None
                }
            }
        }
    }

    /// Non-Windows stub - always succeeds
    #[cfg(not(windows))]
    pub fn try_acquire() -> Option<Self> {
        Some(Self {
            _marker: std::marker::PhantomData,
        })
    }
}

#[cfg(windows)]
impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = ReleaseMutex(self.handle);
            let _ = CloseHandle(self.handle);
        }
    }
}

#[cfg(not(windows))]
impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        // No-op on non-Windows
    }
}
