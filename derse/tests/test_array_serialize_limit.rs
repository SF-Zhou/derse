use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

struct Probe(PathBuf);

impl Probe {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "derse-array-serialize-limit-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn cargo(&self, command: &str, binary: &str) -> Output {
        Command::new(env!("CARGO"))
            .args([command, "--offline", "--quiet", "--bin", binary])
            .current_dir(&self.0)
            // A separate target directory avoids locking the outer cargo test.
            .env("CARGO_TARGET_DIR", self.0.join("target"))
            .env("CARGO_TERM_COLOR", "never")
            // These subprocesses test compilation, not runtime coverage.
            .env_remove("RUSTFLAGS")
            .env_remove("CARGO_ENCODED_RUSTFLAGS")
            .env_remove("LLVM_PROFILE_FILE")
            .output()
            .unwrap()
    }
}

impl Drop for Probe {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn array_serialization_rejects_lengths_above_32_at_build_time() {
    let probe = Probe::new();
    let derse = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dependency_path = derse
        .to_str()
        .unwrap()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    fs::write(
        probe.0.join("Cargo.toml"),
        format!(
            "[package]\nname = \"derse-array-limit-probe\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\
             [dependencies]\nderse = {{ path = \"{dependency_path}\" }}\n\
             [workspace]\n"
        ),
    )
    .unwrap();
    let lockfile = derse.parent().unwrap().join("Cargo.lock");
    if lockfile.is_file() {
        fs::copy(lockfile, probe.0.join("Cargo.lock")).unwrap();
    }
    let binaries = probe.0.join("src/bin");
    fs::create_dir_all(&binaries).unwrap();
    for (name, source) in [
        ("valid", include_str!("array_limit/valid.rs")),
        ("method", include_str!("array_limit/method.rs")),
        ("ufcs", include_str!("array_limit/ufcs.rs")),
        ("derive", include_str!("array_limit/derive.rs")),
        ("generic", include_str!("array_limit/generic.rs")),
    ] {
        fs::write(binaries.join(format!("{name}.rs")), source).unwrap();
    }

    let output = probe.cargo("run", "valid");
    assert!(
        output.status.success(),
        "supported array lengths must build and round-trip:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // cargo check (including trybuild's compile_fail) does not instantiate the
    // inline const assertion. Build each call path to exercise code generation.
    for binary in ["method", "ufcs", "derive", "generic"] {
        let output = probe.cargo("build", binary);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{binary} unexpectedly built");
        assert!(stderr.contains("E0080"), "{binary}: {stderr}");
        assert!(
            stderr.contains("array serialization supports at most 32 elements"),
            "{binary}: {stderr}"
        );
    }
}
