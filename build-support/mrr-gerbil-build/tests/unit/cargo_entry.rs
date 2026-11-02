//! Actual Cargo under the Gerbil package environment, with a slow consumer.
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug)]
enum Entry {
    Direct,
    Inherited,
}

struct Receipt {
    status: ExitStatus,
    output: Vec<u8>,
    diagnostics: Vec<u8>,
    pid: u32,
}

fn metadata(workspace: &Path, manifest: &Path, entry: Entry) -> Receipt {
    use std::os::unix::process::CommandExt;
    let cargo = std::env::var_os("CARGO").expect("Cargo test executable");
    let mut command = match entry {
        Entry::Direct => Command::new(&cargo),
        Entry::Inherited => {
            let mut command = Command::new("gerbil");
            command.arg("env");
            command.arg(&cargo);
            command
        }
    };
    let mut child = command
        .current_dir(workspace)
        .args([
            "metadata",
            "--offline",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(manifest)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
        .expect("actual Cargo entry");
    let pid = child.id();
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    std::thread::scope(|scope| {
        let diagnostics = scope.spawn(move || {
            let mut bytes = Vec::new();
            stderr.read_to_end(&mut bytes).unwrap();
            bytes
        });
        let mut output = Vec::new();
        let mut byte = [0];
        // Wait for the actual JSON, not interpreter startup diagnostics.
        while stdout.read(&mut byte).unwrap() == 1 {
            output.push(byte[0]);
            if byte[0] == b'{' {
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(200));
        stdout.read_to_end(&mut output).unwrap();
        Receipt {
            status: child.wait().unwrap(),
            output,
            diagnostics: diagnostics.join().unwrap(),
            pid,
        }
    })
}

fn verbose_build(workspace: &Path, manifest: &Path, entry: Entry, variant: &str) -> Receipt {
    use std::os::unix::process::CommandExt;
    let cargo = std::env::var_os("CARGO").unwrap();
    let mut command = match entry {
        Entry::Direct => Command::new(&cargo),
        Entry::Inherited => {
            let mut command = Command::new("gerbil");
            command.arg("env").arg(&cargo);
            command
        }
    };
    let (mut reader, writer) = std::io::pipe().unwrap();
    let mut child = command
        .current_dir(workspace)
        .args(["build", "-vv", "--offline", "--manifest-path"])
        .arg(manifest)
        .env("PROBE_RUN", variant)
        .env(
            "CARGO_TARGET_DIR",
            manifest.parent().unwrap().join("target"),
        )
        .stdin(Stdio::null())
        .stdout(writer.try_clone().unwrap())
        .stderr(writer)
        .process_group(0)
        .spawn()
        .unwrap();
    // Command retains its configured writers; drop it before waiting for EOF.
    drop(command);
    let mut output = Vec::new();
    let mut byte = [0];
    let marker = b"CARGO-EAGAIN-PAYLOAD";
    while reader.read(&mut byte).unwrap() == 1 {
        output.push(byte[0]);
        if output.ends_with(marker) {
            break;
        }
    }
    std::thread::sleep(Duration::from_millis(200));
    reader.read_to_end(&mut output).unwrap();
    Receipt {
        pid: child.id(),
        status: child.wait().unwrap(),
        output,
        diagnostics: Vec::new(),
    }
}

#[test]
#[ignore = "actual Gerbil package dependencies and unchanged five-second idle supervisor required"]
fn cargo_backpressure_pair_preserves_host_parallelism_and_reaps_processes() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let evidence =
        std::env::temp_dir().join(format!("mrr-cargo-entry-{}-{stamp}", std::process::id()));
    std::fs::create_dir_all(evidence.join("src")).unwrap();
    std::fs::write(evidence.join("src/lib.rs"), "").unwrap();
    let description = "cargo-metadata-payload ".repeat(65_536);
    let manifest = evidence.join("Cargo.toml");
    std::fs::write(&manifest, format!(
        "[package]\nname=\"cargo-backpressure-fixture\"\nversion=\"0.0.0\"\nedition=\"2024\"\ndescription={description:?}\n[workspace]\n"
    )).unwrap();
    let workers = std::thread::available_parallelism().unwrap().get().max(4);
    eprintln!(
        "CARGO-ENTRY workers={workers} evidence={}",
        evidence.display()
    );
    let direct = metadata(&workspace, &manifest, Entry::Direct);
    assert!(direct.status.success());
    assert!(direct.output.len() > description.len());
    let inherited = metadata(&workspace, &manifest, Entry::Inherited);
    std::fs::write(evidence.join("inherited.stdout"), &inherited.output).unwrap();
    std::fs::write(evidence.join("inherited.stderr"), &inherited.diagnostics).unwrap();
    assert!(
        inherited.output != direct.output,
        "negative control must reproduce incomplete output, even if the native command returns zero; observed={} expected={}",
        inherited.output.len(),
        direct.output.len()
    );
    eprintln!(
        "CARGO-ENTRY negative-control status={} bytes={}",
        inherited.status,
        inherited.output.len()
    );
    let receipts = std::thread::scope(|scope| {
        let jobs: Vec<_> = (0..workers)
            .map(|_| scope.spawn(|| metadata(&workspace, &manifest, Entry::Direct)))
            .collect();
        jobs.into_iter()
            .map(|job| job.join().unwrap())
            .collect::<Vec<_>>()
    });
    for receipt in &receipts {
        assert!(
            receipt.status.success(),
            "{}",
            String::from_utf8_lossy(&receipt.diagnostics)
        );
        let start = receipt
            .output
            .iter()
            .position(|byte| *byte == b'{')
            .unwrap();
        let complete = &receipt.output[start..] == direct.output.as_slice();
        if !complete {
            std::fs::write(
                evidence.join(format!("native-{}.output", receipt.pid)),
                &receipt.output,
            )
            .unwrap();
        }
        assert!(
            complete,
            "native metadata mismatch: pid={} observed={} expected={} evidence={}",
            receipt.pid,
            receipt.output.len() - start,
            direct.output.len(),
            evidence.display()
        );
    }
    let verbose = evidence.join("verbose");
    std::fs::create_dir_all(verbose.join("src")).unwrap();
    std::fs::write(verbose.join("src/lib.rs"), "").unwrap();
    std::fs::write(verbose.join("Cargo.toml"), "[package]\nname=\"verbose-backpressure-fixture\"\nversion=\"0.0.0\"\nedition=\"2024\"\n[workspace]\n").unwrap();
    std::fs::write(verbose.join("build.rs"), "fn main() { println!(\"cargo:rerun-if-env-changed=PROBE_RUN\"); println!(\"cargo:warning=CARGO-EAGAIN-PAYLOAD-{}\", \"w\".repeat(1048576)); }\n").unwrap();
    let verbose_manifest = verbose.join("Cargo.toml");
    let verbose_direct = verbose_build(&workspace, &verbose_manifest, Entry::Direct, "native");
    assert!(verbose_direct.status.success());
    let verbose_inherited =
        verbose_build(&workspace, &verbose_manifest, Entry::Inherited, "wrapped");
    std::fs::write(
        evidence.join("verbose-inherited.output"),
        &verbose_inherited.output,
    )
    .unwrap();
    assert!(
        !verbose_inherited.status.success(),
        "actual Cargo verbose output must reproduce a nonzero write refusal"
    );
    eprintln!(
        "CARGO-ENTRY verbose-negative status={} bytes={}",
        verbose_inherited.status,
        verbose_inherited.output.len()
    );
    let verbose_recovered =
        verbose_build(&workspace, &verbose_manifest, Entry::Direct, "native-again");
    assert!(verbose_recovered.status.success());
    assert!(
        verbose_recovered
            .output
            .windows(1048576)
            .any(|bytes| bytes.iter().all(|byte| *byte == b'w'))
    );
    let groups: Vec<_> = receipts
        .iter()
        .map(|receipt| receipt.pid)
        .chain([
            direct.pid,
            inherited.pid,
            verbose_direct.pid,
            verbose_inherited.pid,
            verbose_recovered.pid,
        ])
        .collect();
    let snapshot = Command::new("ps")
        .args(["-Ao", "pid=,pgid=,stat="])
        .output()
        .unwrap();
    assert!(snapshot.status.success());
    for line in String::from_utf8(snapshot.stdout).unwrap().lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() == 3 {
            let group: u32 = fields[1].parse().unwrap();
            assert!(
                !groups.contains(&group),
                "owned process remains after natural exit: {line}"
            );
        }
    }
    let mut result = std::fs::File::create(evidence.join("result")).unwrap();
    writeln!(
        result,
        "PASS workers={workers} native-bytes={} owned-groups-absent={}",
        direct.output.len(),
        groups.len()
    )
    .unwrap();
    eprintln!(
        "CARGO-ENTRY PASS workers={workers} complete-bytes={}",
        direct.output.len()
    );
}
