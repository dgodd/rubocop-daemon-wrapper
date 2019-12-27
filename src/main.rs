extern crate fs2;
mod project_root;

use fs2::FileExt;
use scopeguard::guard;
use std::env::{self, args};
use std::io::{self, stdout, Read, Write};
// use std::time::Duration;
use std::net::Shutdown;
// use std::thread::sleep;
use std::fs::{self, OpenOptions};
use std::net::TcpStream;
use std::path::Path;
use std::process::Command;
use which::which;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> std::io::Result<()> {
    let command_prefix = if env::vars().any(|(key, _)| key == "RUBOCOP_DAEMON_USE_BUNDLER") {
        "bundle exec "
    } else {
        ""
    };
    if which("rubocop-daemon").is_err() {
        let args: Vec<String> = env::args().collect();
        let command = format!("{}rubocop", command_prefix);
        Command::new(command).args(args).status()?;
        return Ok(());
    }

    let cache_dir = Path::new(&env::var("HOME").expect("HOME env var"))
        .join(".cache")
        .join("rubocop-daemon");
    fs::create_dir_all(&cache_dir)?;

    let lock_path = cache_dir.join("running.lock");
    lock_file(lock_path.to_str().unwrap()).unwrap();
    guard(lock_path, |path| {
        fs::remove_file(path).unwrap();
    });

    let current_dir = env::current_dir()?.to_str().unwrap().to_string();
    let project_root_dir = project_root::project_root(&current_dir).unwrap_or(current_dir);
    let project_cache_key = project_root_dir.trim_start_matches('/').replace("/", "+");
    let project_cache_dir = cache_dir.join(project_cache_key);
    if !project_cache_dir.join("token").exists() {
        Command::new(format!("{}rubocop-daemon", command_prefix))
            .arg("start")
            .env("CACHE_DIR", &cache_dir)
            .env("PROJECT_CACHE_DIR", &project_cache_dir)
            .env("TOKEN_PATH", project_cache_dir.join("token"))
            .env("PORT_PATH", project_cache_dir.join("port"))
            .env("STDIN_PATH", project_cache_dir.join("stdin"))
            .env("STATUS_PATH", project_cache_dir.join("status"))
            .env("LOCK_PATH", cache_dir.join("running.lock"))
            .status()
            .expect("rubocop failed");
    }

    let stdin_content = env::var("STDIN_CONTENT").unwrap_or_else(|_e| {
        if args().any(|a| a == "--stdin" || a == "-s") {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer).unwrap();
            buffer
        } else {
            "".into()
        }
    });

    let token = fs::read_to_string(project_cache_dir.join("token"))?;
    let port: u16 = fs::read_to_string(project_cache_dir.join("port"))?
        .parse()
        .expect("parsing port file");
    let mut stream = TcpStream::connect(("localhost", port))?;

    let command = format!(
        "{} {} exec {}\n{}",
        token,
        project_root_dir,
        args().skip(1).collect::<Vec<String>>().join(" "),
        stdin_content,
    );
    fs::remove_file(project_cache_dir.join("status")).unwrap_or(());
    stream.write(command.as_bytes())?;
    stream.flush()?;
    stream
        .shutdown(Shutdown::Write)
        .expect("shutdown call failed");

    io::copy(&mut stream, &mut stdout())?;
    match fs::read_to_string(project_cache_dir.join("status"))?.parse() {
        Ok(status) => {
            fs::remove_file(project_cache_dir.join("status")).unwrap();
            std::process::exit(status);
        }
        Err(_) => {
            eprintln!("rubocop-daemon-wrapper: server did not write status to $STATUS_PATH!");
            std::process::exit(1);
        }
    }
}

fn lock_file(lock_path: &str) -> Result<()> {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(lock_path)?;

    file.lock_exclusive()?; // block until this process can lock the file
    Ok(())
}
