use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Append a log entry to logfile.txt in the current directory.
fn append_log(entry: &str) {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let line = format!("{} - {}\n", ts, entry);
    if let Err(e) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("logfile.txt")
        .and_then(|mut f| f.write_all(line.as_bytes()))
    {
        eprintln!("Warning: failed to write logfile: {}", e);
    }
}

/// Rejects unsafe filenames
fn is_safe_path_component(name: &str) -> bool {
    if name.contains("..") { return false; }
    if name.starts_with('/') { return false; }
    if name.len() > 2 {
        let bytes = name.as_bytes();
        if (bytes[0] as char).is_ascii_alphabetic() && bytes[1] == b':' &&
            (bytes[2] == b'\\' || bytes[2] == b'/') {
            return false;
        }
    }
    true
}

/// Build a PathBuf inside `base` from `user_name`.
fn resolved_path_in_base(base: &Path, user_name: &str) -> io::Result<PathBuf> {
    if !is_safe_path_component(user_name) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "unsafe path"));
    }
    Ok(base.join(user_name))
}

/// Copy file
fn copy_file(src: &Path, dst: &Path) -> io::Result<u64> {
    fs::copy(src, dst)
}

/// Backup a file
fn backup_file(base: &Path, filename: &str) -> io::Result<PathBuf> {
    let src = resolved_path_in_base(base, filename)?;
    if !src.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "source not found"));
    }
    let bak_name = format!("{}.bak", filename);
    let dst = resolved_path_in_base(base, &bak_name)?;
    copy_file(&src, &dst)?;
    append_log(&format!("backup {} -> {}", filename, bak_name));
    Ok(dst)
}

/// Restore a file
fn restore_file(base: &Path, filename: &str) -> io::Result<PathBuf> {
    let bak_name = format!("{}.bak", filename);
    let bak = resolved_path_in_base(base, &bak_name)?;
    if !bak.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "backup not found"));
    }
    let dst = resolved_path_in_base(base, filename)?;
    copy_file(&bak, &dst)?;
    append_log(&format!("restore {} <- {}", filename, bak_name));
    Ok(dst)
}

/// Delete a file
fn delete_file(base: &Path, filename: &str) -> io::Result<()> {
    let p = resolved_path_in_base(base, filename)?;
    if !p.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "file not found"));
    }
    fs::remove_file(&p)?;
    append_log(&format!("delete {}", filename));
    Ok(())
}

/// Prompt user input
fn prompt(msg: &str) -> io::Result<String> {
    print!("{}", msg);
    io::stdout().flush()?;
    let mut s = String::new();
    io::stdin().read_line(&mut s)?;
    Ok(s.trim().to_string())
}

/// Pause for Enter key (keeps console open)
fn wait_for_enter() {
    print!("Press Enter to exit...");
    let _ = io::stdout().flush();
    let mut dummy = String::new();
    let _ = io::stdin().read_line(&mut dummy);
}

fn main() -> io::Result<()> {
    let base = std::env::current_dir()?;

    println!("=== SafeBackup (Rust) ===");

    let filename = prompt("Please enter your file name: ")?;
    if !is_safe_path_component(&filename) {
        eprintln!("Error: unsafe filename detected.");
        append_log(&format!("rejected unsafe filename input: {}", filename));
        wait_for_enter();
        std::process::exit(1);
    }

    let command = prompt("Please enter your command (backup, restore, delete): ")?;
    match command.as_str() {
        "backup" => match backup_file(&base, &filename) {
            Ok(dst) => println!("Your backup created: {}", dst.file_name().unwrap().to_string_lossy()),
            Err(e) => { eprintln!("Failed to create backup: {}", e); wait_for_enter(); std::process::exit(1); }
        },
        "restore" => match restore_file(&base, &filename) {
            Ok(_) => println!("File restored: {}", filename),
            Err(e) => { eprintln!("Failed to restore: {}", e); wait_for_enter(); std::process::exit(1); }
        },
        "delete" => match delete_file(&base, &filename) {
            Ok(_) => println!("File deleted: {}", filename),
            Err(e) => { eprintln!("Failed to delete: {}", e); wait_for_enter(); std::process::exit(1); }
        },
        _ => { eprintln!("Invalid command."); wait_for_enter(); std::process::exit(1); }
    }

    wait_for_enter(); // keep console open after successful operation
    Ok(())
}
