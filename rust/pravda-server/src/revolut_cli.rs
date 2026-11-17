//! Explicit local console commands for Revolut X read-only credentials.
//! This path is deliberately separate from all WebView and HTTP handlers.
use std::ffi::OsString;

enum Command {
    Setup(OsString),
    SetupInteractive,
    Balances,
    OrderDetails(OsString),
}

fn parse_command(args: &[OsString]) -> Option<Result<Command, &'static str>> {
    let command = args.first()?.to_str()?;
    match command {
        "--revolut-x-setup" if args.len() == 2 && !args[1].is_empty() => {
            Some(Ok(Command::Setup(args[1].clone())))
        }
        // The desktop assistant launches this mode with no argument: the
        // operator types the file path in the assistant window, never in the
        // WebView, and no path is ever accepted from JavaScript.
        "--revolut-x-setup" if args.is_empty() || args.len() == 1 => {
            Some(Ok(Command::SetupInteractive))
        }
        "--revolut-x-balances" if args.len() == 1 => Some(Ok(Command::Balances)),
        "--revolut-x-order-status" if args.len() == 2 && !args[1].is_empty() => {
            Some(Ok(Command::OrderDetails(args[1].clone())))
        }
        "--revolut-x-setup" | "--revolut-x-balances" | "--revolut-x-order-status" => Some(Err(
            "Usage: --revolut-x-setup [private-pem-path] OR --revolut-x-balances OR --revolut-x-order-status <venue-uuid>",
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
            Command::SetupInteractive | Command::Balances => {}
        }
        Some(Err("Revolut X credential management requires Windows"))
    }
    #[cfg(windows)]
    {
        Some(windows::run(command).await)
    }
}

/// True only for the assistant launched by the desktop button: that console
/// window belongs to this process and closes as soon as the command returns.
fn owns_assistant_window(args: &[OsString]) -> bool {
    matches!(parse_command(args), Some(Ok(Command::SetupInteractive)))
}

/// Keeps the GUI-launched assistant window on screen until the operator presses
/// Enter, after the outcome has been printed. A no-op for every other command,
/// for a terminal-provided path and on platforms without the assistant.
pub fn hold_assistant_window(args: &[OsString]) {
    let holds = owns_assistant_window(args);
    #[cfg(windows)]
    if holds {
        windows::hold_window();
    }
    // Off Windows the assistant does not exist; the predicate is still evaluated
    // so the command vocabulary stays identical on every platform.
    #[cfg(not(windows))]
    let _ = holds;
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
        os::windows::{ffi::OsStringExt, fs::OpenOptionsExt, io::AsRawHandle},
        path::{Path, PathBuf},
    };
    use windows_sys::Win32::{
        Foundation::INVALID_HANDLE_VALUE,
        Storage::FileSystem::{GetFileType, FILE_TYPE_UNKNOWN},
        System::{
            Com::CoTaskMemFree,
            Console::{
                AllocConsole, AttachConsole, GetConsoleWindow, GetStdHandle, SetStdHandle,
                ATTACH_PARENT_PROCESS, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
            },
        },
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

    // The desktop executable is built with `windows_subsystem = "windows"`: it
    // starts with no standard handles, even when CREATE_NEW_CONSOLE gave it a
    // console window. Without this, `print!`, `println!` and `std::io::stdin()`
    // fail silently and the assistant cannot be used. Attach or allocate a
    // console explicitly instead of relying on the creation flag alone.
    fn ensure_console() -> Result<(), &'static str> {
        // SAFETY: these calls only affect this process; no pointer is read.
        unsafe {
            if GetConsoleWindow().is_null()
                && AllocConsole() == 0
                && AttachConsole(ATTACH_PARENT_PROCESS) == 0
            {
                return Err("A Windows console is required");
            }
        }
        bind_standard_handle(STD_INPUT_HANDLE, "CONIN$")?;
        bind_standard_handle(STD_OUTPUT_HANDLE, "CONOUT$")?;
        bind_standard_handle(STD_ERROR_HANDLE, "CONOUT$")?;
        Ok(())
    }

    // Bind one standard handle to the console only when it is absent or
    // invalid. An existing redirection (terminal pipe, test harness) is kept
    // exactly as it is, so captured output never changes destination.
    fn bind_standard_handle(identifier: u32, device: &str) -> Result<(), &'static str> {
        // SAFETY: GetStdHandle and GetFileType only inspect this process.
        let current = unsafe { GetStdHandle(identifier) };
        if !current.is_null()
            && current != INVALID_HANDLE_VALUE
            && unsafe { GetFileType(current) } != FILE_TYPE_UNKNOWN
        {
            return Ok(());
        }
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(device)
            .map_err(|_| "A Windows console is required")?;
        // SAFETY: the handle is valid and stays open for the whole process.
        if unsafe { SetStdHandle(identifier, file.as_raw_handle()) } == 0 {
            return Err("A Windows console is required");
        }
        std::mem::forget(file);
        Ok(())
    }

    // The assistant owns its console window: it would close on exit and take
    // the outcome with it. Keep it readable until the operator acknowledges.
    pub(super) fn hold_window() {
        use std::io::Write;
        println!();
        print!("Press Enter to close this assistant.");
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        let _ = std::io::stdin().read_line(&mut line);
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

    // Interactive path entry for the GUI-launched assistant. The path is not a
    // secret, but it is still never supplied by the WebView.
    fn prompt_key_path() -> Result<PathBuf, &'static str> {
        use std::io::Write;
        print!("Revolut X private key file (PEM) path: ");
        std::io::stdout()
            .flush()
            .map_err(|_| "A Windows console is required")?;
        let mut line = String::new();
        std::io::stdin()
            .read_line(&mut line)
            .map_err(|_| "A Windows console is required")?;
        let trimmed = line.trim().trim_matches('"').trim();
        if trimmed.is_empty() || trimmed.len() > 1024 {
            return Err("Private key file path is invalid");
        }
        Ok(PathBuf::from(trimmed))
    }

    fn prompt_and_store(
        account: &RevolutXVaultAccount<WindowsCredentialStore>,
        pem: &[u8],
    ) -> Result<(), &'static str> {
        let key = Zeroizing::new(
            rpassword::prompt_password("Revolut X API key (hidden): ")
                .map_err(|_| "A Windows console is required")?,
        );
        account
            .provision_new(key.as_bytes(), pem)
            .map_err(|_| "Credentials were not saved: invalid, existing or vault unavailable")?;
        println!("Credentials saved in Windows Credential Manager. No connection was attempted.");
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
        // Every console command needs readable output, and setup needs a usable
        // stdin, including when the GUI executable started this process.
        ensure_console()?;
        let root = trusted_local_root()?;
        authenticate(&root.join("pravda_auth.json"))?;
        let account = RevolutXVaultAccount::new(WindowsCredentialStore)
            .map_err(|_| "Cannot initialize the Revolut X client")?;
        match command {
            Command::Setup(path) => {
                let _provision_lock = acquire_provision_lock(&root)?;
                let pem = read_private_key(Path::new(&path))?;
                prompt_and_store(&account, &pem)?;
            }
            Command::SetupInteractive => {
                let _provision_lock = acquire_provision_lock(&root)?;
                let path = prompt_key_path()?;
                let pem = read_private_key(&path)?;
                prompt_and_store(&account, &pem)?;
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
            Some(Ok(Command::SetupInteractive))
        ));
        assert!(matches!(
            parse_command(&args(&["--revolut-x-setup", "key.pem", "extra"])),
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

    // Only the no-argument assistant owns its console window. A terminal command
    // or an explicit path must never block on an acknowledgement prompt.
    #[test]
    fn only_the_gui_assistant_holds_its_own_window() {
        assert!(owns_assistant_window(&args(&["--revolut-x-setup"])));
        assert!(!owns_assistant_window(&args(&[
            "--revolut-x-setup",
            "key.pem"
        ])));
        assert!(!owns_assistant_window(&args(&["--revolut-x-balances"])));
        assert!(!owns_assistant_window(&args(&[
            "--revolut-x-order-status",
            "uuid"
        ])));
        assert!(!owns_assistant_window(&args(&["--revolut-x-setup", ""])));
        assert!(!owns_assistant_window(&[]));
        assert!(!owns_assistant_window(&args(&["--other"])));
        hold_assistant_window(&args(&["--revolut-x-balances"]));
        hold_assistant_window(&[]);
    }
}
