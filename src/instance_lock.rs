//! Process-wide startup lock for Tempo.
//!
//! Holding [`InstanceLock`] keeps the current process registered as Tempo's
//! only running instance. The platform-specific handle is released on drop.

use crate::{error::Result, persistence::SqliteStore};

#[cfg(any(target_os = "macos", target_os = "windows"))]
use fs2::FileExt;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
use single_instance::SingleInstance;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use std::fs::{File, OpenOptions};
#[cfg(target_os = "windows")]
use std::sync::Arc;

#[cfg(target_os = "windows")]
use slint::ComponentHandle;
#[cfg(target_os = "windows")]
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0},
    System::Threading::{CreateEventW, SetEvent, WaitForSingleObject, INFINITE},
};

#[cfg(target_os = "windows")]
const ACTIVATION_EVENT_NAME: &str = "Local\\Tempo-io.github.thwolter.task-tracker-activate";

/// An owned guard proving that this process acquired Tempo's startup lock.
pub(crate) struct InstanceLock {
    _platform_lock: PlatformLock,
}

enum PlatformLock {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    File {
        _file: File,
        #[cfg(target_os = "windows")]
        activation: WindowsActivation,
    },
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    System { _instance: SingleInstance },
}

/// Acquires Tempo's single-instance lock.
///
/// Returns `Ok(None)` when another Tempo process already holds the lock.
pub(crate) fn acquire() -> Result<Option<InstanceLock>> {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        #[cfg(target_os = "windows")]
        let activation = WindowsActivation::create()?;

        let data_dir = SqliteStore::default_data_dir();
        std::fs::create_dir_all(&data_dir)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(data_dir.join("tempo.lock"))?;

        match file.try_lock_exclusive() {
            Ok(()) => Ok(Some(InstanceLock {
                _platform_lock: PlatformLock::File {
                    _file: file,
                    #[cfg(target_os = "windows")]
                    activation,
                },
            })),
            Err(error) if error.kind() == fs2::lock_contended_error().kind() => {
                #[cfg(target_os = "windows")]
                activation.signal()?;
                Ok(None)
            }
            Err(error) => Err(error.into()),
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let instance = SingleInstance::new("com.thomas.tempo")?;
        Ok(instance.is_single().then_some(InstanceLock {
            _platform_lock: PlatformLock::System {
                _instance: instance,
            },
        }))
    }
}

impl InstanceLock {
    /// Starts the Windows reactivation listener after Tempo's main window exists.
    ///
    /// A later launch signals the named event before it exits. The listener uses
    /// Slint's event-loop hand-off, so showing and focusing the window remains on
    /// the UI thread.
    pub(crate) fn listen_for_activation(&self, ui: &crate::AppWindow) -> Result<()> {
        #[cfg(target_os = "windows")]
        let PlatformLock::File { activation, .. } = &self._platform_lock;

        #[cfg(target_os = "windows")]
        activation.listen(ui.as_weak())?;

        #[cfg(not(target_os = "windows"))]
        let _ = ui;

        Ok(())
    }
}

#[cfg(target_os = "windows")]
struct WindowsActivation {
    event: Arc<NamedEvent>,
}

#[cfg(target_os = "windows")]
impl WindowsActivation {
    fn create() -> std::io::Result<Self> {
        let mut name: Vec<u16> = ACTIVATION_EVENT_NAME.encode_utf16().collect();
        name.push(0);
        let handle = unsafe { CreateEventW(std::ptr::null(), 0, 0, name.as_ptr()) };
        if handle.is_null() {
            return Err(std::io::Error::last_os_error());
        }
        Ok(Self {
            event: Arc::new(NamedEvent(handle)),
        })
    }

    fn signal(&self) -> std::io::Result<()> {
        if unsafe { SetEvent(self.event.0) } == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }

    fn listen(&self, ui: slint::Weak<crate::AppWindow>) -> std::io::Result<()> {
        let event = self.event.clone();
        std::thread::Builder::new()
            .name("tempo-activation-listener".into())
            .spawn(move || loop {
                if unsafe { WaitForSingleObject(event.0, INFINITE) } != WAIT_OBJECT_0 {
                    return;
                }
                let _ = ui.upgrade_in_event_loop(restore_and_focus);
            })?;
        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn restore_and_focus(ui: crate::AppWindow) {
    use i_slint_backend_winit::WinitWindowAccessor;

    let _ = ui.show();
    ui.window()
        .with_winit_window(|window| window.focus_window());
}

/// A Win32 event handle is safe to signal and wait from separate threads. Its
/// `Arc` keeps the kernel object open until the process exits.
#[cfg(target_os = "windows")]
struct NamedEvent(HANDLE);

#[cfg(target_os = "windows")]
unsafe impl Send for NamedEvent {}
#[cfg(target_os = "windows")]
unsafe impl Sync for NamedEvent {}

#[cfg(target_os = "windows")]
impl Drop for NamedEvent {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.0) };
    }
}
