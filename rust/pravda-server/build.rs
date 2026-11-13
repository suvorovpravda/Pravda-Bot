// Package the exact Windows release binaries inside a single-click tester.
// Plain cargo check/test builds may use empty placeholders, but release packaging
// MUST set PRAVDA_TEST_CENTER_REQUIRE_PAYLOADS=1 and supply all three PE files.
use std::{env, fs, path::PathBuf};
fn main() {
    println!("cargo:rerun-if-env-changed=PRAVDA_TEST_CENTER_PAYLOAD_DIR");
    println!("cargo:rerun-if-env-changed=PRAVDA_TEST_CENTER_REQUIRE_PAYLOADS");
    let destination = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let source = env::var_os("PRAVDA_TEST_CENTER_PAYLOAD_DIR").map(PathBuf::from);
    let required = env::var("PRAVDA_TEST_CENTER_REQUIRE_PAYLOADS").as_deref() == Ok("1");
    for name in ["pravda-app.exe", "pravda-doctor.exe", "pravda-harness.exe"] {
        let target = destination.join(name);
        let origin = source.as_ref().map(|dir| dir.join(name));
        if let Some(source_path) = &origin {
            println!("cargo:rerun-if-changed={}", source_path.display());
        }
        if let Some(file) = origin.as_ref().filter(|p| p.is_file()) {
            let bytes = fs::read(file).expect("Cannot read test center payload");
            if required && (bytes.len() < 100_000 || !bytes.starts_with(b"MZ")) {
                panic!("Production test payload missing or invalid: {name}");
            }
            fs::write(&target, bytes).expect("Cannot embed test payload");
        } else if required {
            panic!("Production test center requires release payload: {name}");
        } else {
            fs::write(&target, []).expect("Cannot create check-only placeholder");
        }
    }
    // The user-facing setup is a distinct EXE. It embeds a full offline
    // MSI and the verified one-click diagnostic rather than opening a
    // six-second desktop smoke window as if it were the installed app.
    println!("cargo:rerun-if-env-changed=PRAVDA_SETUP_PAYLOAD_DIR");
    println!("cargo:rerun-if-env-changed=PRAVDA_SETUP_REQUIRE_PAYLOADS");
    let setup_dir = env::var_os("PRAVDA_SETUP_PAYLOAD_DIR").map(PathBuf::from);
    let setup_required = env::var("PRAVDA_SETUP_REQUIRE_PAYLOADS").as_deref() == Ok("1");
    // A single executable must contain BOTH the fully offline MSI and the
    // independent self-contained diagnostic center (Doctor + Harness + GUI
    // smoke tests). A missing payload is a hard release-build failure.
    // Development cargo check can still use empty placeholders.
    for (name, file_name, header, minimum_size) in [
        (
            "pravda-embedded-installer.msi",
            "pravda-installer.msi",
            b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1".as_slice(),
            100_000,
        ),
        (
            "pravda-embedded-test-center.exe",
            "pravda-test-center.exe",
            b"MZ".as_slice(),
            1_000_000,
        ),
    ] {
        let destination_file = destination.join(name);
        let origin = setup_dir.as_ref().map(|dir| dir.join(file_name));
        if let Some(file) = &origin {
            println!("cargo:rerun-if-changed={}", file.display());
        }
        if let Some(file) = origin.filter(|file| file.is_file()) {
            let bytes = fs::read(&file).expect("Cannot read unified setup payload");
            if setup_required && (bytes.len() < minimum_size || !bytes.starts_with(header)) {
                panic!("Required Windows setup payload invalid: {file_name}");
            }
            fs::write(destination_file, bytes).expect("Cannot embed Windows setup payload");
        } else if setup_required {
            panic!("Required Windows setup payload missing: {file_name}");
        } else {
            fs::write(destination_file, []).expect("Cannot create setup check-only placeholder");
        }
    }
}
