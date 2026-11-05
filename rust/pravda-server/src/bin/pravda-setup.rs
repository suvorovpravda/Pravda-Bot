#![cfg_attr(windows, windows_subsystem = "windows")]
// Single-file Windows installer. The installed desktop owns its native Rust
// pre/postproduction test center; no separate diagnostic must be launched.
// No exchange access, private-profile data migration, or live trading.

#[cfg(windows)]
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(windows)]
const MSI: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/pravda-embedded-installer.msi"));
#[cfg(any(windows, test))]
fn valid_msi(bytes: &[u8]) -> bool {
    const HEADER: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    bytes.len() > 100_000 && bytes.starts_with(&HEADER)
}

#[cfg(windows)]
struct TemporaryBundle {
    directory: PathBuf,
}

#[cfg(windows)]
impl TemporaryBundle {
    fn create() -> io::Result<Self> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("pravda-setup-{}-{nanos}", std::process::id()));
        // Never overwrite an existing path or work in a real user profile.
        fs::create_dir(&directory)?;
        Ok(Self { directory })
    }

    fn write(&self, name: &str, bytes: &[u8]) -> io::Result<PathBuf> {
        let path = self.directory.join(name);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        // Verify the actual extracted bytes, not just a filename or extension.
        if fs::read(&path)?.as_slice() != bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Embedded payload differs from extracted file",
            ));
        }
        Ok(path)
    }
}

#[cfg(windows)]
impl Drop for TemporaryBundle {
    fn drop(&mut self) {
        // Only remove our uniquely created temporary files, never user data.
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[cfg(windows)]
fn dialog(title: &str, content: &str, choices: bool) -> i32 {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, MB_ICONINFORMATION, MB_OK, MB_YESNOCANCEL,
    };
    let title: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let content: Vec<u16> = content.encode_utf16().chain(std::iter::once(0)).collect();
    let flags = MB_ICONINFORMATION | if choices { MB_YESNOCANCEL } else { MB_OK };
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            content.as_ptr(),
            title.as_ptr(),
            flags,
        )
    }
}

#[cfg(windows)]
fn find_installed_app() -> Option<PathBuf> {
    // Standard Windows MSI target paths. A custom installation path may need
    // launching from the Start menu; do not guess or execute unrelated files.
    for variable in ["ProgramFiles", "ProgramW6432", "LOCALAPPDATA"] {
        let Some(root) = std::env::var_os(variable) else {
            continue;
        };
        let mut root = PathBuf::from(root);
        if variable == "LOCALAPPDATA" {
            root.push("Programs");
        }
        let root = root.join("Pravda-Bot");
        for name in ["pravda-app.exe", "Pravda-Bot.exe"] {
            let exe = root.join(name);
            if exe.is_file() {
                return Some(exe);
            }
        }
    }
    None
}

#[cfg(windows)]
fn install(bundle: &TemporaryBundle) -> Result<ExitStatus, &'static str> {
    let msi = bundle
        .write("Pravda-Bot.msi", MSI)
        .map_err(|_| "Cannot stage offline MSI")?;
    // Windows Installer owns the normal installation UI and consent/UAC.
    // No unattended elevation, registry modification, or security bypass.
    let system_root = std::env::var_os("SystemRoot").ok_or("Windows system root is missing")?;
    let msiexec = PathBuf::from(system_root).join("System32").join("msiexec.exe");
    if !msiexec.is_file() {
        return Err("Windows Installer is unavailable");
    }
    Command::new(msiexec)
        .arg("/i")
        .arg(&msi)
        .arg("/norestart")
        .status()
        .map_err(|_| "Windows Installer could not start")
}

#[cfg(windows)]
fn run() -> Result<(), &'static str> {
    use windows_sys::Win32::UI::WindowsAndMessaging::IDYES;
    if !valid_msi(MSI) {
        return Err("The embedded Windows installer is missing or invalid");
    }
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.len() == 1 && arguments[0] == "--verify-only" {
        // Used by independent CI: no writes, installs, profiles or network.
        return Ok(());
    }
    if !arguments.is_empty() {
        return Err("Unsupported argument");
    }
    // Preflight is read-only; Windows Installer owns all installation changes.
    if std::env::consts::ARCH != "x86_64" || std::env::var_os("SystemRoot").is_none() {
        return Err("Windows x64 is required");
    }
    let choice = dialog(
        "Pravda-Bot — Installation Windows",
        "Un seul installateur pour Pravda-Bot avec WebView2 hors ligne.\n\
         \n\
         OUI : installer l'application. Le centre de tests pre/postproduction\n\
         est ensuite accessible dans Parametres, apres connexion.\n\
         NON / ANNULER : quitter sans modifier le PC.\n\
         \n\
         Ce candidat de test n'est pas une edition Enterprise signee.\n\
         Le choix de modules facultatifs reste a qualifier.",
        true,
    );
    if choice != IDYES {
        return Ok(());
    }
    let bundle = TemporaryBundle::create().map_err(|_| "Cannot create setup workspace")?;
    let status = install(&bundle)?;
    match status.code() {
        Some(0) => {}
        Some(3010) => {
            dialog(
                "Pravda-Bot",
                "L'installation demande un redémarrage Windows. Redémarre normalement avant d'ouvrir l'application.",
                false,
            );
            return Ok(());
        }
        _ => {
            return Err("MSI installation failed or was cancelled; no diagnostics were substituted")
        }
    }
    let choice = dialog(
        "Pravda-Bot — Installation terminee",
        "L'installation est terminee.\n\
         \n\
         OUI : ouvrir l'application sans limite de duree.\n\
         NON / ANNULER : terminer ; utiliser ensuite le menu Demarrer.\n\
         \n\
         Une fois connecte, ouvrir Parametres > Securite et depannage\n\
         pour acceder aux campagnes de tests pre/postproduction.",
        true,
    );
    if choice == IDYES {
        if let Some(executable) = find_installed_app() {
            Command::new(executable)
                .spawn()
                .map_err(|_| "The installed Pravda desktop could not start")?;
        } else {
            dialog(
                "Pravda-Bot",
                "Le dossier standard n'a pas été trouvé. Ouvre Pravda-Bot depuis le menu Démarrer ; les fichiers personnels n'ont pas été modifiés.",
                false,
            );
        }
    }
    Ok(())
}

#[cfg(windows)]
fn main() {
    let is_verify = std::env::args_os().skip(1).any(|s| s == "--verify-only");
    if let Err(reason) = run() {
        if !is_verify {
            dialog("Pravda-Bot — Erreur", reason, false);
        }
        std::process::exit(2);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("Pravda-Setup requires Windows.");
}

#[cfg(test)]
mod tests {
    use super::valid_msi;

    #[test]
    fn rejects_invalid_and_empty_installation_payloads() {
        assert!(!valid_msi(b"MZ"));
        assert!(!valid_msi(b""));
    }

    #[test]
    fn accepts_only_expected_container_headers() {
        let mut installer = vec![0u8; 100_001];
        installer[..8].copy_from_slice(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
        assert!(valid_msi(&installer));
        installer[..2].copy_from_slice(b"MZ");
        assert!(!valid_msi(&installer));
    }
}
