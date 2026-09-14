use std::env;
use std::ffi::CStr;
use std::fs;
use std::io::Read;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use chrono::{Local, TimeZone};

// ---------------------------------------------------------------------
// echo
// ---------------------------------------------------------------------

pub fn echo(args: &[String]) {
    println!("{}", args.join(" "));
}

// ---------------------------------------------------------------------
// pwd
// ---------------------------------------------------------------------

pub fn pwd(_args: &[String]) {
    match env::current_dir() {
        Ok(path) => println!("{}", path.display()),
        Err(e) => eprintln!("pwd: {}", e),
    }
}

// ---------------------------------------------------------------------
// cd
// ---------------------------------------------------------------------

pub fn cd(args: &[String]) {
    let target: PathBuf = if args.is_empty() {
        match env::var("HOME") {
            Ok(home) => PathBuf::from(home),
            Err(_) => {
                eprintln!("cd: HOME not set");
                return;
            }
        }
    } else {
        PathBuf::from(&args[0])
    };

    if let Err(e) = env::set_current_dir(&target) {
        eprintln!("cd: {}: {}", target.display(), describe_io_error(&e));
    }
}

// ---------------------------------------------------------------------
// mkdir
// ---------------------------------------------------------------------

pub fn mkdir(args: &[String]) {
    if args.is_empty() {
        eprintln!("mkdir: missing operand");
        return;
    }

    for dir in args {
        if let Err(e) = fs::create_dir(dir) {
            eprintln!("mkdir: cannot create directory '{}': {}", dir, describe_io_error(&e));
        }
    }
}

// ---------------------------------------------------------------------
// cat
// ---------------------------------------------------------------------

pub fn cat(args: &[String]) {
    if args.is_empty() {
        eprintln!("cat: missing operand");
        return;
    }

    for file in args {
        match fs::File::open(file) {
            Ok(mut f) => {
                let mut contents = Vec::new();
                if let Err(e) = f.read_to_end(&mut contents) {
                    eprintln!("cat: {}: {}", file, e);
                    continue;
                }
                use std::io::Write;
                let stdout = std::io::stdout();
                let mut handle = stdout.lock();
                let _ = handle.write_all(&contents);
            }
            Err(e) => eprintln!("cat: {}: {}", file, describe_io_error(&e)),
        }
    }
}

// ---------------------------------------------------------------------
// cp
// ---------------------------------------------------------------------

pub fn cp(args: &[String]) {
    if args.len() != 2 {
        eprintln!("cp: usage: cp <source> <destination>");
        return;
    }

    let src = Path::new(&args[0]);
    let mut dst = PathBuf::from(&args[1]);

    if dst.is_dir() {
        if let Some(name) = src.file_name() {
            dst.push(name);
        }
    }

    if let Err(e) = fs::copy(src, &dst) {
        eprintln!("cp: cannot copy '{}' to '{}': {}", src.display(), dst.display(), describe_io_error(&e));
    }
}

// ---------------------------------------------------------------------
// mv
// ---------------------------------------------------------------------

pub fn mv(args: &[String]) {
    if args.len() != 2 {
        eprintln!("mv: usage: mv <source> <destination>");
        return;
    }

    let src = Path::new(&args[0]);
    let mut dst = PathBuf::from(&args[1]);

    if dst.is_dir() {
        if let Some(name) = src.file_name() {
            dst.push(name);
        }
    }

    if let Err(e) = fs::rename(src, &dst) {
        eprintln!("mv: cannot move '{}' to '{}': {}", src.display(), dst.display(), describe_io_error(&e));
    }
}

// ---------------------------------------------------------------------
// rm
// ---------------------------------------------------------------------

pub fn rm(args: &[String]) {
    let mut recursive = false;
    let mut targets: Vec<&String> = Vec::new();

    for arg in args {
        if arg == "-r" || arg == "-R" {
            recursive = true;
        } else {
            targets.push(arg);
        }
    }

    if targets.is_empty() {
        eprintln!("rm: missing operand");
        return;
    }

    for target in targets {
        let path = Path::new(target);
        let metadata = match fs::symlink_metadata(path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("rm: cannot remove '{}': {}", target, describe_io_error(&e));
                continue;
            }
        };

        let result = if metadata.is_dir() {
            if recursive {
                fs::remove_dir_all(path)
            } else {
                eprintln!("rm: cannot remove '{}': Is a directory", target);
                continue;
            }
        } else {
            fs::remove_file(path)
        };

        if let Err(e) = result {
            eprintln!("rm: cannot remove '{}': {}", target, describe_io_error(&e));
        }
    }
}

// ---------------------------------------------------------------------
// ls
// ---------------------------------------------------------------------

pub fn ls(args: &[String]) {
    let mut show_long = false;
    let mut show_all = false;
    let mut classify = false;
    let mut paths: Vec<&String> = Vec::new();

    for arg in args {
        if let Some(flags) = arg.strip_prefix('-') {
            if flags.is_empty() {
                continue;
            }
            for c in flags.chars() {
                match c {
                    'l' => show_long = true,
                    'a' => show_all = true,
                    'F' => classify = true,
                    _ => eprintln!("ls: invalid option -- '{}'", c),
                }
            }
        } else {
            paths.push(arg);
        }
    }

    if paths.is_empty() {
        list_dir(".", show_long, show_all, classify);
    } else {
        for p in &paths {
            if paths.len() > 1 {
                println!("{}:", p);
            }
            list_dir(p, show_long, show_all, classify);
        }
    }
}

fn list_dir(dir: &str, show_long: bool, show_all: bool, classify: bool) {
    let path = Path::new(dir);

    let metadata = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("ls: cannot access '{}': {}", dir, describe_io_error(&e));
            return;
        }
    };

    // If the target is a plain file, just list that single entry.
    if !metadata.is_dir() {
        print_entries(&[(dir.to_string(), metadata)], show_long, classify);
        return;
    }

    let read_dir = match fs::read_dir(path) {
        Ok(rd) => rd,
        Err(e) => {
            eprintln!("ls: cannot access '{}': {}", dir, describe_io_error(&e));
            return;
        }
    };

    let mut entries: Vec<(String, fs::Metadata)> = Vec::new();

    if show_all {
        if let Ok(m) = fs::symlink_metadata(path) {
            entries.push((".".to_string(), m));
        }
        let parent = path.join("..");
        if let Ok(m) = fs::symlink_metadata(&parent) {
            entries.push(("..".to_string(), m));
        }
    }

    for entry in read_dir.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !show_all && name.starts_with('.') {
            continue;
        }
        if let Ok(m) = entry.metadata() {
            entries.push((name, m));
        }
    }

    entries.sort_by(|a, b| a.0.cmp(&b.0));

    print_entries(&entries, show_long, classify);
}

fn print_entries(entries: &[(String, fs::Metadata)], show_long: bool, classify: bool) {
    if show_long {
        let total: u64 = entries.iter().map(|(_, m)| m.blocks()).sum::<u64>() / 2;
        println!("total {}", total);

        for (name, meta) in entries {
            println!("{}", format_long_entry(name, meta, classify));
        }
    } else {
        let names: Vec<String> = entries
            .iter()
            .map(|(name, meta)| format!("{}{}", name, classify_suffix(meta, classify)))
            .collect();
        println!("{}", names.join("  "));
    }
}

fn classify_suffix(meta: &fs::Metadata, classify: bool) -> &'static str {
    if !classify {
        return "";
    }
    if meta.is_dir() {
        "/"
    } else if meta.file_type().is_symlink() {
        "@"
    } else if meta.permissions().mode() & 0o111 != 0 {
        "*"
    } else {
        ""
    }
}

fn format_long_entry(name: &str, meta: &fs::Metadata, classify: bool) -> String {
    let mode = meta.permissions().mode();
    let perms = format_permissions(mode, meta.file_type());
    let nlink = meta.nlink();
    let owner = user_name(meta.uid());
    let group = group_name(meta.gid());
    let size = meta.size();
    let time = format_time(meta.mtime());
    let suffix = classify_suffix(meta, classify);

    format!(
        "{} {:>3} {:<8} {:<8} {:>8} {} {}{}",
        perms, nlink, owner, group, size, time, name, suffix
    )
}

fn format_permissions(mode: u32, file_type: fs::FileType) -> String {
    let type_char = if file_type.is_dir() {
        'd'
    } else if file_type.is_symlink() {
        'l'
    } else if file_type.is_char_device() {
        'c'
    } else if file_type.is_block_device() {
        'b'
    } else if file_type.is_fifo() {
        'p'
    } else if file_type.is_socket() {
        's'
    } else {
        '-'
    };

    let bits = [
        (mode & 0o400, 'r'), (mode & 0o200, 'w'), (mode & 0o100, 'x'),
        (mode & 0o040, 'r'), (mode & 0o020, 'w'), (mode & 0o010, 'x'),
        (mode & 0o004, 'r'), (mode & 0o002, 'w'), (mode & 0o001, 'x'),
    ];

    let mut s = String::with_capacity(10);
    s.push(type_char);
    for (bit, ch) in bits {
        s.push(if bit != 0 { ch } else { '-' });
    }
    s
}

fn format_time(mtime: i64) -> String {
    match Local.timestamp_opt(mtime, 0).single() {
        Some(dt) => dt.format("%b %e %H:%M").to_string(),
        None => String::from("??? ?? ??:??"),
    }
}

// ---------------------------------------------------------------------
// uid/gid -> name resolution (using libc, no external binaries spawned)
// ---------------------------------------------------------------------

fn user_name(uid: u32) -> String {
    unsafe {
        let pw = libc::getpwuid(uid);
        if pw.is_null() {
            return uid.to_string();
        }
        let name_ptr = (*pw).pw_name;
        if name_ptr.is_null() {
            return uid.to_string();
        }
        CStr::from_ptr(name_ptr).to_string_lossy().into_owned()
    }
}

fn group_name(gid: u32) -> String {
    unsafe {
        let gr = libc::getgrgid(gid);
        if gr.is_null() {
            return gid.to_string();
        }
        let name_ptr = (*gr).gr_name;
        if name_ptr.is_null() {
            return gid.to_string();
        }
        CStr::from_ptr(name_ptr).to_string_lossy().into_owned()
    }
}

// ---------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------

fn describe_io_error(e: &std::io::Error) -> String {
    match e.kind() {
        std::io::ErrorKind::NotFound => "No such file or directory".to_string(),
        std::io::ErrorKind::PermissionDenied => "Permission denied".to_string(),
        std::io::ErrorKind::AlreadyExists => "File exists".to_string(),
        _ => e.to_string(),
    }
}
