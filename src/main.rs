// directory dusting: du -s -c -h * | sort

use clap::Parser;
use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "dust",
    about = "directory dusting, similar to: du -s -c -h * | sort",
    version
)]
struct Options {
    /// only include directories when listing the current directory
    #[arg(short = 'd')]
    directory_only: bool,

    /// files or directories to measure (default: contents of current directory)
    list: Vec<String>,
}

fn default_list(directory_only: bool) -> std::io::Result<Vec<String>> {
    let mut list = Vec::new();
    for entry in fs::read_dir(".")? {
        let entry = entry?;
        if directory_only && !entry.file_type()?.is_dir() {
            continue;
        }
        list.push(entry.file_name().to_string_lossy().into_owned());
    }
    Ok(list)
}

#[cfg(unix)]
fn disk_usage(_path: &Path, meta: &fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    meta.blocks() * 512
}

#[cfg(windows)]
fn disk_usage(path: &Path, meta: &fs::Metadata) -> u64 {
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        FileStandardInfo, GetFileInformationByHandleEx, FILE_FLAG_OPEN_REPARSE_POINT,
        FILE_STANDARD_INFO,
    };

    let mut options = OpenOptions::new();
    options.read(true);
    if meta.file_type().is_symlink() {
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }

    let file = match options.open(path) {
        Ok(file) => file,
        Err(_) => return meta.len(),
    };

    let mut info: FILE_STANDARD_INFO = unsafe { std::mem::zeroed() };
    let ok = unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle() as _,
            FileStandardInfo,
            &mut info as *mut _ as *mut _,
            std::mem::size_of::<FILE_STANDARD_INFO>() as u32,
        )
    };

    if ok == 0 {
        meta.len()
    } else {
        info.AllocationSize as u64
    }
}

#[cfg(not(any(unix, windows)))]
fn disk_usage(_path: &Path, meta: &fs::Metadata) -> u64 {
    meta.len()
}

fn dir_size(path: &Path) -> u64 {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(_) => return 0,
    };

    if !meta.is_dir() {
        return disk_usage(path, &meta);
    }

    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return 0,
    };

    entries.flatten().map(|entry| dir_size(&entry.path())).sum()
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 7] = ["B", "K", "M", "G", "T", "P", "E"];

    if bytes < 1024 {
        return format!("{bytes}");
    }

    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }

    if size < 10.0 {
        format!("{size:.1}{}", UNITS[unit])
    } else {
        format!("{size:.0}{}", UNITS[unit])
    }
}

fn run(args: Vec<String>) -> ExitCode {
    let opts = match Options::try_parse_from(std::iter::once("dust".to_string()).chain(args)) {
        Ok(opts) => opts,
        Err(err) => {
            let _ = err.print();
            return ExitCode::from(err.exit_code() as u8);
        }
    };

    let list = if opts.list.is_empty() {
        match default_list(opts.directory_only) {
            Ok(list) => list,
            Err(err) => {
                eprintln!("unable to read . {err}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        opts.list
    };

    let mut sizes: Vec<(String, u64)> = Vec::new();
    for name in list {
        match fs::symlink_metadata(&name) {
            Ok(_) => {
                let size = dir_size(Path::new(&name));
                sizes.push((name, size));
            }
            Err(err) => eprintln!("{name}: {err}"),
        }
    }

    sizes.sort_by_key(|(_, size)| *size);

    for (name, size) in sizes {
        println!("{}\t{name}", human_bytes(size));
    }

    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    run(env::args().skip(1).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_bytes_formats_common_sizes() {
        assert_eq!(human_bytes(0), "0");
        assert_eq!(human_bytes(1023), "1023");
        assert_eq!(human_bytes(1024), "1.0K");
        assert_eq!(human_bytes(1536), "1.5K");
        assert_eq!(human_bytes(10 * 1024), "10K");
        assert_eq!(human_bytes(1024 * 1024), "1.0M");
        assert_eq!(human_bytes(5 * 1024 * 1024 * 1024), "5.0G");
    }

    fn parse(args: &[&str]) -> Options {
        Options::try_parse_from(std::iter::once("dust").chain(args.iter().copied())).unwrap()
    }

    #[test]
    fn parse_args_sets_directory_only() {
        let opts = parse(&["-d"]);
        assert!(opts.directory_only);
        assert!(opts.list.is_empty());
    }

    #[test]
    fn parse_args_collects_positional_list() {
        let opts = parse(&["foo", "bar"]);
        assert!(!opts.directory_only);
        assert_eq!(opts.list, vec!["foo".to_string(), "bar".to_string()]);
    }

    #[test]
    fn parse_args_double_dash_stops_option_parsing() {
        let opts = parse(&["-d", "--", "-weird", "name"]);
        assert!(opts.directory_only);
        assert_eq!(opts.list, vec!["-weird".to_string(), "name".to_string()]);
    }

    #[test]
    fn parse_args_rejects_unknown_option() {
        let err = Options::try_parse_from(["dust", "-x"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::UnknownArgument);
    }
}
