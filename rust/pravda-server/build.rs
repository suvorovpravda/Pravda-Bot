// Package the exact Windows release binaries inside a single-click tester.
// Plain cargo check/test builds may use empty placeholders, but release packaging
// MUST set PRAVDA_TEST_CENTER_REQUIRE_PAYLOADS=1 and supply all three PE files.
use std::{env,fs,path::PathBuf};
fn main() {
    println!("cargo:rerun-if-env-changed=PRAVDA_TEST_CENTER_PAYLOAD_DIR");
    println!("cargo:rerun-if-env-changed=PRAVDA_TEST_CENTER_REQUIRE_PAYLOADS");
    let destination=PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let source=env::var_os("PRAVDA_TEST_CENTER_PAYLOAD_DIR").map(PathBuf::from);
    let required=env::var("PRAVDA_TEST_CENTER_REQUIRE_PAYLOADS").as_deref()==Ok("1");
    for name in ["pravda-app.exe","pravda-doctor.exe","pravda-harness.exe"] {
        let target=destination.join(name);
        let origin=source.as_ref().map(|dir|dir.join(name));
        if let Some(file)=origin.as_ref().filter(|p|p.is_file()) {
            let bytes=fs::read(file).expect("Cannot read test center payload");
            if required && (bytes.len()<100_000||!bytes.starts_with(b"MZ")) {
                panic!("Production test payload missing or invalid: {name}");
            }
            fs::write(&target,bytes).expect("Cannot embed test payload");
        } else if required {
            panic!("Production test center requires release payload: {name}");
        } else {
            fs::write(&target,[]).expect("Cannot create check-only placeholder");
        }
    }
}
