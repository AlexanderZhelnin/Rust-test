use std::{
    env,
    error::Error,
    ffi::OsString,
    fs,
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Barrier},
    thread,
    time::{Duration, Instant},
};

type DynError = Box<dyn Error + Send + Sync>;
type Result<T> = std::result::Result<T, DynError>;

const PACKAGE: &str = "api_test";
const BINARY: &str = "api_test";
const SERVER_ADDRESS: &str = "127.0.0.1:3003";
const PGO_TRAINING_ENV: &str = "API_TEST_PGO_TRAINING";
const SMOKE_ENDPOINTS: [&str; 0/*4*/] = [
    // "/",
    // "/readfile",
    // "/fibonacci",
    // "/naturalsortHack"
    ];
const MAP_REQUESTS: [&str; 0/*6*/] = [
    // "/map",
    // "/map?x=1&y=1",
    // "/map?x=25&y=25",
    // "/map?x=50&y=50",
    // "/map?x=75&y=75",
    // "/map?x=100&y=100",
];
const MAP_JSON_REQUESTS: [&str; 0/*6*/] = [
    // "/mapJSON",
    // "/mapJSON?x=1&y=1",
    // "/mapJSON?x=25&y=25",
    // "/mapJSON?x=50&y=50",
    // "/mapJSON?x=75&y=75",
    // "/mapJSON?x=100&y=100",
];
const NATURAL_SORT_REQUESTS: [&str; 1] = ["/naturalsort"];
const TRAINING_WORKLOADS: [TrainingWorkload; 1/*3*/] = [
    // TrainingWorkload {
    //     name: "/map",
    //     requests: &MAP_REQUESTS,
    // },
    // TrainingWorkload {
    //     name: "/mapJSON",
    //     requests: &MAP_JSON_REQUESTS,
    // },
    TrainingWorkload {
        name: "/naturalsort",
        requests: &NATURAL_SORT_REQUESTS,
    },
];

#[derive(Clone, Copy)]
struct TrainingWorkload {
    name: &'static str,
    requests: &'static [&'static str],
}

#[derive(Clone, Copy)]
struct PgoOptions {
    workers: usize,
    warmup: Duration,
    training: Duration,
}

impl Default for PgoOptions {
    fn default() -> Self {
        Self {
            workers: 100,
            warmup: Duration::from_secs(1),
            training: Duration::from_secs(5),
        }
    }
}

#[derive(Default)]
struct LoadStats {
    completed: u64,
    failures: u64,
    elapsed: Duration,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("xtask: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("pgo") => run_pgo(parse_pgo_options(args)?),
        Some("help" | "--help" | "-h") | None => {
            print_help();
            Ok(())
        }
        Some(command) => Err(message(format!(
            "unknown command `{command}`; run `cargo xtask --help`"
        ))),
    }
}

fn print_help() {
    println!(
        "\
Project build tasks

USAGE:
    cargo xtask pgo [OPTIONS]

OPTIONS:
    --workers <N>            Parallel HTTP connections [default: 100]
    --warmup-seconds <N>     Warm-up per endpoint [default: 1]
    --training-seconds <N>   Profile collection per endpoint [default: 5]
    -h, --help               Print help

The final executable is written to target/pgo/api_test-pgo{suffix}",
        suffix = env::consts::EXE_SUFFIX
    );
}

fn parse_pgo_options(mut args: impl Iterator<Item = String>) -> Result<PgoOptions> {
    let mut options = PgoOptions::default();

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--workers" => {
                options.workers = parse_number(&argument, args.next())?;
                if options.workers == 0 {
                    return Err(message("--workers must be greater than zero"));
                }
            }
            "--warmup-seconds" => {
                let seconds: u64 = parse_number(&argument, args.next())?;
                options.warmup = Duration::from_secs(seconds);
            }
            "--training-seconds" => {
                let seconds: u64 = parse_number(&argument, args.next())?;
                if seconds == 0 {
                    return Err(message("--training-seconds must be greater than zero"));
                }
                options.training = Duration::from_secs(seconds);
            }
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            _ => return Err(message(format!("unknown PGO option `{argument}`"))),
        }
    }

    Ok(options)
}

fn parse_number<T>(option: &str, value: Option<String>) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: Error + Send + Sync + 'static,
{
    value
        .ok_or_else(|| message(format!("{option} requires a value")))?
        .parse::<T>()
        .map_err(|error| Box::new(error) as DynError)
}

fn run_pgo(options: PgoOptions) -> Result<()> {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| message("xtask has no workspace parent"))?
        .to_path_buf();
    let host = rustc_host(&workspace)?;
    let llvm_profdata = ensure_llvm_profdata(&workspace, &host)?;

    let pgo_root = workspace.join("target").join("pgo");
    if pgo_root.exists() {
        fs::remove_dir_all(&pgo_root)?;
    }

    let profiles = pgo_root.join("profiles");
    let instrumented_target = pgo_root.join("instrumented-target");
    let optimized_target = pgo_root.join("optimized-target");
    let merged_profile = pgo_root.join("merged.profdata");
    fs::create_dir_all(&profiles)?;

    println!("[1/6] Building the instrumented executable");
    cargo_build(
        &workspace,
        &host,
        &instrumented_target,
        &[
            "-C",
            "target-cpu=native",
            "-C",
            &format!("profile-generate={}", profiles.to_string_lossy()),
        ],
    )?;

    ensure_server_port_is_free()?;
    let instrumented_executable = built_executable(&instrumented_target, &host);

    train_instrumented_server(&instrumented_executable, &workspace, options)?;

    let raw_profiles = collect_raw_profiles(&profiles)?;
    if raw_profiles.is_empty() {
        return Err(message(format!(
            "LLVM did not create any .profraw files in {}",
            profiles.display()
        )));
    }

    println!("[4/6] Merging {} raw profile(s)", raw_profiles.len());
    merge_profiles(&llvm_profdata, &merged_profile, &raw_profiles, &workspace)?;

    println!("[5/6] Building the optimized executable");
    cargo_build(
        &workspace,
        &host,
        &optimized_target,
        &[
            "-C",
            "target-cpu=native",
            "-C",
            &format!("profile-use={}", merged_profile.to_string_lossy()),
        ],
    )?;

    let optimized_executable = built_executable(&optimized_target, &host);
    if !optimized_executable.is_file() {
        return Err(message(format!(
            "optimized executable was not produced at {}",
            optimized_executable.display()
        )));
    }

    let final_executable = pgo_root.join(format!("{BINARY}-pgo{}", env::consts::EXE_SUFFIX));
    fs::copy(&optimized_executable, &final_executable)?;

    println!("[6/6] PGO build is ready");
    println!("Executable: {}", final_executable.display());
    println!("Merged profile: {}", merged_profile.display());
    Ok(())
}

fn cargo_build(workspace: &Path, host: &str, target_dir: &Path, rustflags: &[&str]) -> Result<()> {
    let mut command = Command::new(cargo_program());
    command
        .current_dir(workspace)
        .args([
            "build",
            "--release",
            "--locked",
            "--package",
            PACKAGE,
            "--bin",
            BINARY,
            "--target",
            host,
        ])
        .env("CARGO_TARGET_DIR", target_dir)
        .env_remove("RUSTFLAGS")
        .env("CARGO_ENCODED_RUSTFLAGS", rustflags.join("\x1f"));
    run_checked(&mut command, "cargo build")
}

fn rustc_host(workspace: &Path) -> Result<String> {
    let mut command = Command::new(rustc_program());
    command.current_dir(workspace).arg("-vV");
    let output = capture_checked(&mut command, "rustc -vV")?;
    output
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(str::to_owned)
        .ok_or_else(|| message("rustc -vV did not report a host triple"))
}

fn ensure_llvm_profdata(workspace: &Path, host: &str) -> Result<PathBuf> {
    let mut command = Command::new(rustc_program());
    command.current_dir(workspace).args(["--print", "sysroot"]);
    let sysroot = PathBuf::from(capture_checked(&mut command, "rustc --print sysroot")?.trim());
    let tool = sysroot
        .join("lib")
        .join("rustlib")
        .join(host)
        .join("bin")
        .join(format!("llvm-profdata{}", env::consts::EXE_SUFFIX));

    if !tool.is_file() {
        println!("Installing the rustup component llvm-tools-preview");
        let mut rustup = Command::new("rustup");
        rustup
            .current_dir(workspace)
            .args(["component", "add", "llvm-tools-preview"]);
        run_checked(&mut rustup, "rustup component add llvm-tools-preview")?;
    }

    if !tool.is_file() {
        return Err(message(format!(
            "llvm-profdata was not found at {}; install Rust through rustup",
            tool.display()
        )));
    }

    Ok(tool)
}

fn train_instrumented_server(
    executable: &Path,
    workspace: &Path,
    options: PgoOptions,
) -> Result<()> {
    if !executable.is_file() {
        return Err(message(format!(
            "instrumented executable was not produced at {}",
            executable.display()
        )));
    }

    let mut child = Command::new(executable)
        .current_dir(workspace)
        .env(PGO_TRAINING_ENV, "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    let training_result = (|| {
        wait_until_ready(&mut child, Duration::from_secs(30))?;

        println!("[2/6] Smoke-checking the remaining endpoints");
        for endpoint in SMOKE_ENDPOINTS {
            connect_and_request(endpoint)?;
            println!("  smoke {endpoint}");
        }

        println!("[3/6] Training on /map, /mapJSON and /naturalsort");
        for workload in TRAINING_WORKLOADS {
            if !options.warmup.is_zero() {
                let warmup = run_load(workload.requests, options.warmup, options.workers)?;
                if warmup.failures != 0 {
                    return Err(message(format!(
                        "warm-up for {} had {} failure(s)",
                        workload.name, warmup.failures
                    )));
                }
            }

            let stats = run_load(workload.requests, options.training, options.workers)?;
            if stats.failures != 0 {
                return Err(message(format!(
                    "training for {} had {} failure(s)",
                    workload.name, stats.failures
                )));
            }

            println!(
                "  {:<12} {:>10.2} req/s  ({} requests, {} variant(s))",
                workload.name,
                stats.completed as f64 / stats.elapsed.as_secs_f64(),
                stats.completed,
                workload.requests.len()
            );
        }

        Ok(())
    })();

    let shutdown_result = stop_training_server(&mut child);
    training_result?;
    shutdown_result
}

fn ensure_server_port_is_free() -> Result<()> {
    let listener = TcpListener::bind(SERVER_ADDRESS).map_err(|error| {
        message(format!(
            "{SERVER_ADDRESS} is already in use; stop the existing server before PGO training: {error}"
        ))
    })?;
    drop(listener);
    Ok(())
}

fn wait_until_ready(child: &mut Child, timeout: Duration) -> Result<()> {
    let deadline = Instant::now() + timeout;
    let mut last_error = None;

    while Instant::now() < deadline {
        if let Some(status) = child.try_wait()? {
            return Err(message(format!(
                "instrumented server exited before training with {status}"
            )));
        }

        match connect_and_request("/") {
            Ok(()) => return Ok(()),
            Err(error) => last_error = Some(error),
        }
        thread::sleep(Duration::from_millis(50));
    }

    Err(last_error.unwrap_or_else(|| message("server readiness timed out")))
}

fn connect_and_request(endpoint: &str) -> Result<()> {
    let mut stream = new_connection()?;
    let mut buffer = Vec::with_capacity(1024);
    let mut scratch = [0_u8; 64 * 1024];
    send_request(&mut stream, endpoint, &mut buffer, &mut scratch)?;
    Ok(())
}

fn stop_training_server(child: &mut Child) -> Result<()> {
    if let Some(status) = child.try_wait()? {
        return if status.success() {
            Ok(())
        } else {
            Err(message(format!("instrumented server exited with {status}")))
        };
    }

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(b"\n")?;
        stdin.flush()?;
    }

    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if let Some(status) = child.try_wait()? {
            return if status.success() {
                Ok(())
            } else {
                Err(message(format!("instrumented server exited with {status}")))
            };
        }
        thread::sleep(Duration::from_millis(50));
    }

    child.kill()?;
    let _ = child.wait();
    Err(message(
        "instrumented server did not stop cleanly, so its PGO profile may be incomplete",
    ))
}

fn run_load(
    requests: &'static [&'static str],
    duration: Duration,
    workers: usize,
) -> Result<LoadStats> {
    if requests.is_empty() {
        return Err(message("a training workload has no request variants"));
    }

    let barrier = Arc::new(Barrier::new(workers + 1));
    let mut handles = Vec::with_capacity(workers);

    for worker_index in 0..workers {
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            barrier.wait();
            run_worker(requests, worker_index, duration)
        }));
    }

    barrier.wait();
    let started = Instant::now();
    let mut combined = LoadStats::default();

    for handle in handles {
        let stats = handle
            .join()
            .map_err(|_| message("a load-generator worker panicked"))?;
        combined.completed += stats.completed;
        combined.failures += stats.failures;
    }

    combined.elapsed = started.elapsed();
    Ok(combined)
}

fn run_worker(requests: &[&str], worker_index: usize, duration: Duration) -> LoadStats {
    let deadline = Instant::now() + duration;
    let mut stream = None;
    let mut buffer = Vec::with_capacity(512 * 1024);
    let mut scratch = [0_u8; 64 * 1024];
    let mut stats = LoadStats::default();
    let mut request_index = worker_index % requests.len();

    while Instant::now() < deadline {
        if stream.is_none() {
            match new_connection() {
                Ok(connection) => {
                    buffer.clear();
                    stream = Some(connection);
                }
                Err(_) => {
                    stats.failures += 1;
                    thread::yield_now();
                    continue;
                }
            }
        }

        let result = send_request(
            stream.as_mut().expect("connection was created above"),
            requests[request_index],
            &mut buffer,
            &mut scratch,
        );
        request_index = (request_index + 1) % requests.len();

        match result {
            Ok(keep_alive) => {
                stats.completed += 1;
                if !keep_alive {
                    stream = None;
                }
            }
            Err(_) => {
                stats.failures += 1;
                stream = None;
            }
        }
    }

    stats
}

fn new_connection() -> Result<TcpStream> {
    let stream = TcpStream::connect(SERVER_ADDRESS)?;
    stream.set_nodelay(true)?;
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    stream.set_write_timeout(Some(Duration::from_secs(30)))?;
    Ok(stream)
}

fn send_request(
    stream: &mut TcpStream,
    endpoint: &str,
    buffer: &mut Vec<u8>,
    scratch: &mut [u8],
) -> Result<bool> {
    write!(
        stream,
        "GET {endpoint} HTTP/1.1\r\nHost: {SERVER_ADDRESS}\r\nConnection: keep-alive\r\nAccept: */*\r\n\r\n"
    )?;
    stream.flush()?;

    let header_end = loop {
        if let Some(position) = find_bytes(buffer, b"\r\n\r\n") {
            break position;
        }
        if buffer.len() > 64 * 1024 {
            return Err(message("HTTP response headers exceeded 64 KiB"));
        }
        read_more(stream, buffer, scratch)?;
    };

    let header = std::str::from_utf8(&buffer[..header_end])?.to_owned();
    buffer.drain(..header_end + 4);

    let mut lines = header.split("\r\n");
    let status = lines
        .next()
        .ok_or_else(|| message("HTTP response had no status line"))?;
    let status_code = status
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| message("HTTP response had an invalid status line"))?
        .parse::<u16>()?;
    if !(200..300).contains(&status_code) {
        return Err(message(format!(
            "HTTP response returned status {status_code}"
        )));
    }

    let mut content_length = None;
    let mut chunked = false;
    let mut keep_alive = true;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("content-length") {
            content_length = Some(value.trim().parse::<usize>()?);
        } else if name.eq_ignore_ascii_case("transfer-encoding")
            && value.to_ascii_lowercase().contains("chunked")
        {
            chunked = true;
        } else if name.eq_ignore_ascii_case("connection")
            && value.trim().eq_ignore_ascii_case("close")
        {
            keep_alive = false;
        }
    }

    if chunked {
        consume_chunked_body(stream, buffer, scratch)?;
    } else if let Some(length) = content_length {
        ensure_buffered(stream, buffer, scratch, length)?;
        buffer.drain(..length);
    } else {
        return Err(message(
            "HTTP response had neither Content-Length nor chunked encoding",
        ));
    }

    Ok(keep_alive)
}

fn consume_chunked_body(
    stream: &mut TcpStream,
    buffer: &mut Vec<u8>,
    scratch: &mut [u8],
) -> Result<()> {
    loop {
        let line_end = loop {
            if let Some(position) = find_bytes(buffer, b"\r\n") {
                break position;
            }
            read_more(stream, buffer, scratch)?;
        };

        let size_line = std::str::from_utf8(&buffer[..line_end])?;
        let size =
            usize::from_str_radix(size_line.split(';').next().unwrap_or_default().trim(), 16)?;
        buffer.drain(..line_end + 2);

        if size == 0 {
            ensure_buffered(stream, buffer, scratch, 2)?;
            if buffer.starts_with(b"\r\n") {
                buffer.drain(..2);
                return Ok(());
            }

            let trailers_end = loop {
                if let Some(position) = find_bytes(buffer, b"\r\n\r\n") {
                    break position;
                }
                read_more(stream, buffer, scratch)?;
            };
            buffer.drain(..trailers_end + 4);
            return Ok(());
        }

        ensure_buffered(stream, buffer, scratch, size + 2)?;
        if &buffer[size..size + 2] != b"\r\n" {
            return Err(message("chunked HTTP body had an invalid terminator"));
        }
        buffer.drain(..size + 2);
    }
}

fn ensure_buffered(
    stream: &mut TcpStream,
    buffer: &mut Vec<u8>,
    scratch: &mut [u8],
    required: usize,
) -> Result<()> {
    while buffer.len() < required {
        read_more(stream, buffer, scratch)?;
    }
    Ok(())
}

fn read_more(stream: &mut TcpStream, buffer: &mut Vec<u8>, scratch: &mut [u8]) -> Result<()> {
    let read = stream.read(scratch)?;
    if read == 0 {
        return Err(message(
            "HTTP connection closed before the response completed",
        ));
    }
    buffer.extend_from_slice(&scratch[..read]);
    Ok(())
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn collect_raw_profiles(directory: &Path) -> Result<Vec<PathBuf>> {
    let mut profiles = Vec::new();
    collect_files_with_extension(directory, "profraw", &mut profiles)?;
    profiles.sort();
    Ok(profiles)
}

fn collect_files_with_extension(
    directory: &Path,
    extension: &str,
    output: &mut Vec<PathBuf>,
) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_files_with_extension(&path, extension, output)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
            output.push(path);
        }
    }
    Ok(())
}

fn merge_profiles(
    llvm_profdata: &Path,
    output: &Path,
    profiles: &[PathBuf],
    workspace: &Path,
) -> Result<()> {
    let mut command = Command::new(llvm_profdata);
    command
        .current_dir(workspace)
        .arg("merge")
        .arg("-o")
        .arg(output);
    command.args(profiles);
    run_checked(&mut command, "llvm-profdata merge")
}

fn built_executable(target_dir: &Path, host: &str) -> PathBuf {
    target_dir
        .join(host)
        .join("release")
        .join(format!("{BINARY}{}", env::consts::EXE_SUFFIX))
}

fn cargo_program() -> OsString {
    env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"))
}

fn rustc_program() -> OsString {
    env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"))
}

fn run_checked(command: &mut Command, description: &str) -> Result<()> {
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(message(format!("{description} failed with {status}")))
    }
}

fn capture_checked(command: &mut Command, description: &str) -> Result<String> {
    let output = command.output()?;
    if !output.status.success() {
        return Err(message(format!(
            "{description} failed with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn message(text: impl Into<String>) -> DynError {
    Box::new(io::Error::other(text.into()))
}
