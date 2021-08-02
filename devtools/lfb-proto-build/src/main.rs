//! Build LFB/Ostracon/IBC proto files. This build script clones the LFB SDK version
//! specified in the LFB_REV constant and then uses that to build the required
//! proto files for further compilation. This is based on the proto-compiler code
//! in github.com/informalsystems/ibc-rs

use regex::Regex;
use std::{
    env,
    ffi::OsStr,
    fs::{self, create_dir_all, remove_dir_all},
    io,
    path::{Path, PathBuf},
    process,
    sync::atomic::{self, AtomicBool},
};
use walkdir::WalkDir;

/// Suppress log messages
// TODO(tarcieri): use a logger for this
static QUIET: AtomicBool = AtomicBool::new(false);

/// The LFB commit or tag to be cloned and used to build the proto files
const LFB_REV: &str = "af78aa4c1c22da9e76f67b5452a3146df4ed0e15";

// All paths must end with a / and either be absolute or include a ./ to reference the current
// working directory.

/// The directory generated proto files go into in this repo
const LFB_SDK_PROTO_DIR: &str = "../../packages/lfb-sdk-proto/src/prost/";
/// Directory where the submodule is located
const LFB_SDK_DIR: &str = "../lfb-sdk-go";
/// A temporary directory for proto building
const TMP_BUILD_DIR: &str = "/tmp/tmp-protobuf/";

// Patch strings used by `copy_and_patch`

/// Protos belonging to these Protobuf packages will be excluded
/// (i.e. because they are sourced from `lfb-proto`)
const EXCLUDED_PROTO_PACKAGES: &[&str] = &["gogoproto", "google", "ostracon"];
/// Regex for locating instances of `ostracon-proto` in prost/tonic build output
const OSTRACON_PROTO_REGEX: &str = "(super::)+ostracon";
/// Attribute preceeding a Tonic client definition
const TONIC_CLIENT_ATTRIBUTE: &str = "#[doc = r\" Generated client implementations.\"]";
/// Attributes to add to gRPC clients
const GRPC_CLIENT_ATTRIBUTES: &[&str] = &[
    "#[cfg(feature = \"grpc\")]",
    "#[cfg_attr(docsrs, doc(cfg(feature = \"grpc\")))]",
    TONIC_CLIENT_ATTRIBUTE,
];

/// Log info to the console (if `QUIET` is disabled)
// TODO(tarcieri): use a logger for this
macro_rules! info {
    ($msg:expr) => {
        if !is_quiet() {
            println!("[info] {}", $msg)
        }
    };
    ($fmt:expr, $($arg:tt)+) => {
        info!(&format!($fmt, $($arg)+))
    };
}

fn main() {
    if is_github() {
        set_quiet();
    }

    let tmp_build_dir: PathBuf = TMP_BUILD_DIR.parse().unwrap();
    let proto_dir: PathBuf = LFB_SDK_PROTO_DIR.parse().unwrap();

    if tmp_build_dir.exists() {
        fs::remove_dir_all(tmp_build_dir.clone()).unwrap();
    }

    fs::create_dir(tmp_build_dir.clone()).unwrap();

    update_submodule();
    output_sdk_version(&tmp_build_dir);
    compile_protos(&tmp_build_dir);
    compile_proto_services(&tmp_build_dir);
    copy_generated_files(&tmp_build_dir, &proto_dir);

    if is_github() {
        println!(
            "Rebuild protos with proto-build (lfb-sdk rev: {})",
            LFB_REV
        );
    }
}

fn is_quiet() -> bool {
    QUIET.load(atomic::Ordering::Relaxed)
}

fn set_quiet() {
    QUIET.store(true, atomic::Ordering::Relaxed);
}

/// Parse `--github` flag passed to `proto-build` on the eponymous GitHub Actions job.
/// Disables `info`-level log messages, instead outputting only a commit message.
fn is_github() -> bool {
    env::args().any(|arg| arg == "--github")
}

fn run_git(args: impl IntoIterator<Item = impl AsRef<OsStr>>) {
    let stdout = if is_quiet() {
        process::Stdio::null()
    } else {
        process::Stdio::inherit()
    };

    let exit_status = process::Command::new("git")
        .args(args)
        .stdout(stdout)
        .status()
        .expect("git exit status missing");

    if !exit_status.success() {
        panic!("git exited with error code: {:?}", exit_status.code());
    }
}

fn update_submodule() {
    info!("Updating line/lfb-sdk submodule...");
    run_git(&["submodule", "update", "--init"]);
    run_git(&["-C", LFB_SDK_DIR, "fetch"]);
    run_git(&["-C", LFB_SDK_DIR, "reset", "--hard", LFB_REV]);
}

fn output_sdk_version(out_dir: &Path) {
    let path = out_dir.join("LFB_SDK_COMMIT");
    fs::write(path, LFB_REV).unwrap();
}

fn compile_protos(out_dir: &Path) {
    let sdk_dir = Path::new(LFB_SDK_DIR);

    info!(
        "Compiling .proto files to Rust into '{}'...",
        out_dir.display()
    );

    let root = env!("CARGO_MANIFEST_DIR");

    // Paths
    let proto_paths = [
        format!("{}/../proto/definitions/mock", root),
        format!("{}/proto/lfb/auth", sdk_dir.display()),
        format!("{}/proto/lfb/bank", sdk_dir.display()),
        format!("{}/proto/lfb/base", sdk_dir.display()),
        format!("{}/proto/lfb/base/ostracon", sdk_dir.display()),
        format!("{}/proto/lfb/capability", sdk_dir.display()),
        format!("{}/proto/lfb/crisis", sdk_dir.display()),
        format!("{}/proto/lfb/crypto", sdk_dir.display()),
        format!("{}/proto/lfb/distribution", sdk_dir.display()),
        format!("{}/proto/lfb/evidence", sdk_dir.display()),
        format!("{}/proto/lfb/genutil", sdk_dir.display()),
        format!("{}/proto/lfb/gov", sdk_dir.display()),
        format!("{}/proto/lfb/mint", sdk_dir.display()),
        format!("{}/proto/lfb/params", sdk_dir.display()),
        format!("{}/proto/lfb/slashing", sdk_dir.display()),
        format!("{}/proto/lfb/staking", sdk_dir.display()),
        format!("{}/proto/lfb/tx", sdk_dir.display()),
        format!("{}/proto/lfb/upgrade", sdk_dir.display()),
        format!("{}/proto/lfb/vesting", sdk_dir.display()),
        format!("{}/proto/ibc", sdk_dir.display()),
    ];

    let proto_includes_paths = [
        format!("{}/../proto", root),
        format!("{}/proto", sdk_dir.display()),
        format!("{}/third_party/proto", sdk_dir.display()),
    ];

    // List available proto files
    let mut protos: Vec<PathBuf> = vec![];
    for proto_path in &proto_paths {
        protos.append(
            &mut WalkDir::new(proto_path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_type().is_file()
                        && e.path().extension().is_some()
                        && e.path().extension().unwrap() == "proto"
                })
                .map(|e| e.into_path())
                .collect(),
        );
    }

    // List available paths for dependencies
    let includes: Vec<PathBuf> = proto_includes_paths.iter().map(PathBuf::from).collect();

    // Compile all proto files
    let mut config = prost_build::Config::default();
    config.out_dir(out_dir);
    config.extern_path(".ostracon", "::ostracon_proto");

    if let Err(e) = config.compile_protos(&protos, &includes) {
        eprintln!("[error] couldn't compile protos: {}", e);
        panic!("protoc failed!");
    }
}

fn compile_proto_services(out_dir: impl AsRef<Path>) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sdk_dir = PathBuf::from(LFB_SDK_DIR);

    let proto_includes_paths = [
        root.join("../proto"),
        sdk_dir.join("proto"),
        sdk_dir.join("third_party/proto"),
    ];

    // List available paths for dependencies
    let includes = proto_includes_paths
        .iter()
        .map(|p| p.as_os_str().to_os_string())
        .collect::<Vec<_>>();

    let proto_services_path = [
        sdk_dir.join("proto/lfb/auth/v1beta1/query.proto"),
        sdk_dir.join("proto/lfb/bank/v1beta1/query.proto"),
        sdk_dir.join("proto/lfb/bank/v1beta1/tx.proto"),
        sdk_dir.join("proto/lfb/base/ostracon/v1beta1/query.proto"),
        sdk_dir.join("proto/lfb/crisis/v1beta1/tx.proto"),
        sdk_dir.join("proto/lfb/distribution/v1beta1/query.proto"),
        sdk_dir.join("proto/lfb/distribution/v1beta1/tx.proto"),
        sdk_dir.join("proto/lfb/evidence/v1beta1/query.proto"),
        sdk_dir.join("proto/lfb/evidence/v1beta1/tx.proto"),
        sdk_dir.join("proto/lfb/gov/v1beta1/query.proto"),
        sdk_dir.join("proto/lfb/gov/v1beta1/tx.proto"),
        sdk_dir.join("proto/lfb/mint/v1beta1/query.proto"),
        sdk_dir.join("proto/lfb/params/v1beta1/query.proto"),
        sdk_dir.join("proto/lfb/slashing/v1beta1/query.proto"),
        sdk_dir.join("proto/lfb/slashing/v1beta1/tx.proto"),
        sdk_dir.join("proto/lfb/staking/v1beta1/query.proto"),
        sdk_dir.join("proto/lfb/staking/v1beta1/tx.proto"),
        sdk_dir.join("proto/lfb/tx/v1beta1/service.proto"),
        sdk_dir.join("proto/lfb/tx/v1beta1/tx.proto"),
        sdk_dir.join("proto/lfb/upgrade/v1beta1/query.proto"),
        sdk_dir.join("proto/lfb/vesting/v1beta1/tx.proto"),
    ];

    // List available paths for dependencies
    let services = proto_services_path
        .iter()
        .map(|p| p.as_os_str().to_os_string())
        .collect::<Vec<_>>();

    // Compile all proto client for GRPC services
    info!("Compiling proto clients for GRPC services!");
    tonic_build::configure()
        .build_client(true)
        .build_server(false)
        .format(true)
        .out_dir(out_dir)
        .compile(&services, &includes)
        .unwrap();

    info!("=> Done!");
}

fn copy_generated_files(from_dir: &Path, to_dir: &Path) {
    info!("Copying generated files into '{}'...", to_dir.display());

    // Remove old compiled files
    remove_dir_all(&to_dir).unwrap_or_default();
    create_dir_all(&to_dir).unwrap();

    let mut filenames = Vec::new();

    // Copy new compiled files (prost does not use folder structures)
    let errors = WalkDir::new(from_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            let filename = e.file_name().to_os_string().to_str().unwrap().to_string();
            filenames.push(filename.clone());
            copy_and_patch(e.path(), format!("{}/{}", to_dir.display(), &filename))
        })
        .filter_map(|e| e.err())
        .collect::<Vec<_>>();

    if !errors.is_empty() {
        for e in errors {
            eprintln!("[error] Error while copying compiled file: {}", e);
        }

        panic!("[error] Aborted.");
    }
}

fn copy_and_patch(src: impl AsRef<Path>, dest: impl AsRef<Path>) -> io::Result<()> {
    // Skip proto files belonging to `EXCLUDED_PROTO_PACKAGES`
    for package in EXCLUDED_PROTO_PACKAGES {
        if let Some(filename) = src.as_ref().file_name().and_then(OsStr::to_str) {
            if filename.starts_with(&format!("{}.", package)) {
                return Ok(());
            }
        }
    }

    let contents = fs::read_to_string(src)?;

    // `prost-build` output references types from `ostracon-proto` crate
    // relative paths, which we need to munge into `ostracon_proto` in
    // order to leverage types from the upstream crate.
    let contents = Regex::new(OSTRACON_PROTO_REGEX)
        .unwrap()
        .replace_all(&contents, "ostracon_proto");

    // Patch each service definition with a feature attribute
    let patched_contents =
        contents.replace(TONIC_CLIENT_ATTRIBUTE, &GRPC_CLIENT_ATTRIBUTES.join("\n"));

    fs::write(dest, patched_contents)
}
