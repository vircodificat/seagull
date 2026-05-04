use std::path::Path;
use std::process::Command;

pub struct LinuxOpener;

impl super::Opener for LinuxOpener {
    fn open_path(&self, path: &Path) -> std::io::Result<()> {
        Command::new("xdg-open").arg(path).spawn().map(|_| ())
    }
}
