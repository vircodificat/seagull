use std::path::Path;

/// Open files / URLs in the user's default handler. Implementations are
/// platform-specific so that menu actions never reference `xdg-open`,
/// `open`, or `ShellExecuteW` directly.
pub trait Opener: Send + Sync {
    fn open_path(&self, path: &Path) -> std::io::Result<()>;
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::LinuxOpener as DefaultOpener;
