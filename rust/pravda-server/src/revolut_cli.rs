//! Explicit local console commands for Revolut X read-only credentials.
//! This path is deliberately separate from all WebView and HTTP handlers.
use std::ffi::OsString;

enum Command {
    Setup(OsString),
    Balances,
    OrderDetails(OsString),
}

fn parse_command(args: &[OsString]) -> Option<Result<Command, &'static str>> {
    let command = args.first()?.to_str()?;
    match command {
        "--revolut-x-setup" if args.len() == 2 && !args[1].is_empty() => {
            Some(Ok(Command::Setup(args[1].clone())))
        }
        "--revolut-x-balances" if args.len() == 1 => Some(Ok(Command::Balances)),
        "--revolut-x-order-status" if args.len() == 2 && !args[1].is_empty() => {
            Some(Ok(Command::OrderDetails(args[1].clone())))
        }
        "--revolut-x-setup" | "--revolut-x-balances" | "--revolut-x-order-status" => Some(Err(
            "Usage: --revolut-x-setup <private-pem-path> OR --revolut-x-balances OR --revolut-x-order-status <venue-uuid>",
        )),
        _ => None,
    }
}

/// Returns None for ordinary desktop startup and otherwise handles the command.
pub async fn run(args: &[OsString]) -> Option<Result<(), &'static str>> {
    let command = match parse_command(args)? {
        Ok(command) => command,
        Err(error) => return Some(Err(error)),
    };
    #[cfg(not(windows))]
    {
        match command {
            Command::Setup(path) | Command::OrderDetails(path) => drop(path),
            Command::Balances => {}
        }
        Some(Err("Revolut X credential management requires Windows"))
    }
    #[cfg(windows)]
    {
        Some(windows::run(command).await)
    }
}

#[cfg(windows)]
mod windows {
    use super::Command;
    use crate::{
        revolut_x_account::RevolutXVaultAccount, secret_store::WindowsCredentialStore, Auth,
        PBKDF2_ITERATIONS,
    };
    use std::{
        io::Read,
        os::windows::{ffi::OsStringExt, fs::OpenOptionsExt},
        path::{Path, PathBuf},
    };
    use windows_sys::Win32::{
        System::Com::CoTaskMemFree,
        UI::Shell::{FOLDERID_LocalAppData, SHGetKnownFolderPath},
    };
    use zeroize::Zeroizing;

    // Resolve the Windows account's actual folder, not a caller-controlled
    // environment override that could select an unrelated auth record.
    fn trusted_local_root() -> Result<PathBuf, &'static str> {
        let mut raw = std::ptr::null_mut();
        // SAFETY: Windows writes a COM-allocated, NUL-terminated path to raw.
        let result = unsafe {
            SHGetKnownFolderPath(&FOLDERID_LocalAppData, 0, std::ptr::null_mut(), &mut raw)
        };
        if result < 0 || raw.is_null() {
            // SAFETY: CoTaskMemFree accepts both null and the returned pointer.
            unsafe { CoTaskMemFree(raw.cast()) };
            return Err("Cannot locate the Windows account data directory");
        }
        // SAFETY: SHGetKnownFolderPath returned a valid NUL-terminated string.
        let len = unsafe { (0..32768).find(|i| *raw.add(*i) == 0) };
        let path = len.map(|len| {
            // SAFETY: len points to the first NUL in the Windows-owned string.
            let text = unsafe { std::slice::from_raw_parts(raw, len) };
            PathBuf::from(std::ffi::OsString::from_wide(text)).join("Pravda-Bot")
        });
        // SAFETY: raw was allocated by SHGetKnownFolderPath and is freed once.
        unsafe { CoTaskMemFree(raw.cast()) };
        path.ok_or("Windows account data directory is invalid")
    }

    fn authenticate(path: &Path) -> Result<(), &'static str> {
        let auth = Auth::load(path, PBKDF2_ITERATIONS)
            .map_err(|_| "Unable to load the local authentication file")?;
        if !auth.is_configured() {
            return Err("Set up the local application password first");
        }
        let password = Zeroizing::new(
            rpassword::prompt_password("Local Pravda-Bot password: ")
                .map_err(|_| "A Windows console is required")?,
        );
        verify_password(&auth, &password)
    }

    fn verify_password(auth: &Auth, password: &str) -> Result<(), &'static str> {
        let token = Zeroizing::new(
            auth.login(password)
                .map_err(|_| "Local authentication failed")?,
        );
        auth.logout(&token);
        Ok(())
    }

    fn read_private_key(path: &Path) -> Result<Zeroizing<Vec<u8>>, &'static str> {
        let file = std::fs::File::open(path).map_err(|_| "Cannot read the private key file")?;
        let mut bytes = Zeroizing::new(Vec::with_capacity(1025));
        file.take(1025)
            .read_to_end(&mut bytes)
            .map_err(|_| "Cannot read the private key file")?;
        if bytes.is_empty() || bytes.len() > 1024 {
            return Err("Private key file size is invalid");
        }
        Ok(bytes)
    }

    // A stable, empty file with exclusive Windows sharing serializes setup
    // across application processes. The open handle is released on crash;
    // the empty file must stay in place to avoid an unlink/recreate race.
    fn acquire_provision_lock(root: &Path) -> Result<std::fs::File, &'static str> {
        std::fs::create_dir_all(root).map_err(|_| "Cannot prepare the local data directory")?;
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .share_mode(0)
            .open(root.join("revolut-x.lock"))
            .map_err(|_| "Another Revolut X setup is running or the lock is unavailable")
    }

    pub(super) async fn run(command: Command) -> Result<(), &'static str> {
        let root = trusted_local_root()?;
        authenticate(&root.join("pravda_auth.json"))?;
        let account = RevolutXVaultAccount::new(WindowsCredentialStore)
            .map_err(|_| "Cannot initialize the Revolut X client")?;
        match command {
            Command::Setup(path) => {
                let _provision_lock = acquire_provision_lock(&root)?;
                let pem = read_private_key(Path::new(&path))?;
                let key = Zeroizing::new(
                    rpassword::prompt_password("Revolut X API key (hidden): ")
                        .map_err(|_| "A Windows console is required")?,
                );
                account.provision_new(key.as_bytes(), &pem).map_err(|_| {
                    "Credentials were not saved: invalid, existing or vault unavailable"
                })?;
                println!(
                    "Credentials saved in Windows Credential Manager. No connection was attempted."
                );
            }
            Command::Balances => {
                let balances = account.balances().await.map_err(|_| {
                    "Unable to read balances; verify credentials, permissions and network"
                })?;
                println!("Revolut X balances (private):");
                for balance in balances {
                    match balance.staked {
                        Some(staked) => println!(
                            "{}: available={}, reserved={}, staked={}, total={}",
                            balance.currency,
                            balance.available,
                            balance.reserved,
                            staked,
                            balance.total
                        ),
                        None => println!(
                            "{}: available={}, reserved={}, total={}",
                            balance.currency, balance.available, balance.reserved, balance.total
                        ),
                    }
                }
            }
            Command::OrderDetails(id) => {
                let venue_id = id.to_str().ok_or("Invalid exchange order UUID")?;
                // SEC: No WebView route, no POST/DELETE or credential exposure.
                let detail = account.order_details(venue_id).await.map_err(|_| {
                    "Unable to reconcile this order; check venue UUID and API permissions"
                })?;
                println!(
                    "Revolut X read-only status: venue={} client={} symbol={} status={} filled={}/{}",
                    detail.venue_order_id,
                    detail.client_order_id,
                    detail.symbol,
                    detail.status,
                    detail.filled_quantity,
                    detail.quantity
                );
            }
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn console_password_requires_existing_valid_local_password() {
            let path = std::env::temp_dir().join(format!(
                "pravda-revolut-auth-{}-{}.json",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            let auth = Auth::load(&path, 1000).unwrap();
            assert!(verify_password(&auth, "unconfigured-password").is_err());
            auth.set_password("test-password-for-revolut-cli").unwrap();
            assert!(verify_password(&auth, "wrong-password").is_err());
            assert!(verify_password(&auth, "test-password-for-revolut-cli").is_ok());
            std::fs::remove_file(path).unwrap();
        }

        #[test]
        fn windows_setup_lock_excludes_other_handles_and_releases_on_drop() {
            let directory = std::env::temp_dir().join(format!(
                "pravda-revolut-lock-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            let first = acquire_provision_lock(&directory).unwrap();
            assert!(acquire_provision_lock(&directory).is_err());
            drop(first);
            assert!(acquire_provision_lock(&directory).is_ok());
            std::fs::remove_dir_all(directory).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(parts: &[&str]) -> Vec<OsString> {
        parts.iter().map(OsString::from).collect()
    }

    #[test]
    fn accepts_only_explicit_commands_and_expected_arguments() {
        assert!(parse_command(&[]).is_none());
        assert!(parse_command(&args(&["--other"])).is_none());
        assert!(matches!(
            parse_command(&args(&["--revolut-x-setup", "key.pem"])),
            Some(Ok(Command::Setup(_)))
        ));
        assert!(matches!(
            parse_command(&args(&["--revolut-x-balances"])),
            Some(Ok(Command::Balances))
        ));
        assert!(matches!(
            parse_command(&args(&["--revolut-x-setup"])),
            Some(Err(_))
        ));
        assert!(matches!(
            parse_command(&args(&["--revolut-x-setup", ""])),
            Some(Err(_))
        ));
        assert!(matches!(
            parse_command(&args(&["--revolut-x-balances", "extra"])),
            Some(Err(_))
        ));
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn fails_closed_without_windows_vault() {
        let result = run(&args(&["--revolut-x-balances"])).await;
        assert!(matches!(
            result,
            Some(Err("Revolut X credential management requires Windows"))
        ));
    }
}
