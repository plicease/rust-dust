// directory dusting: du -s -c -h * | sort

use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

#[derive(Debug)]
struct Options {
    directory_only: bool,
    list: Vec<String>,
}

fn parse_args(args: Vec<String>) -> Result<Options, String> {
    let mut directory_only = false;
    let mut list = Vec::new();
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        if arg == "-d" {
            directory_only = true;
        } else if arg == "--" {
            list.extend(iter);
            break;
        } else if arg.starts_with('-') {
            return Err(format!("unknown option: {arg}"));
        } else {
            list.push(arg);
        }
    }

    Ok(Options {
        directory_only,
        list,
    })
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

fn dir_size(path: &Path) -> u64 {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(_) => return 0,
    };

    if !meta.is_dir() {
        return meta.len();
    }

    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return 0,
    };

    entries
        .flatten()
        .map(|entry| dir_size(&entry.path()))
        .sum()
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
    let opts = match parse_args(args) {
        Ok(opts) => opts,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
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

    #[test]
    fn parse_args_sets_directory_only() {
        let opts = parse_args(vec!["-d".to_string()]).unwrap();
        assert!(opts.directory_only);
        assert!(opts.list.is_empty());
    }

    #[test]
    fn parse_args_collects_positional_list() {
        let opts = parse_args(vec!["foo".to_string(), "bar".to_string()]).unwrap();
        assert!(!opts.directory_only);
        assert_eq!(opts.list, vec!["foo".to_string(), "bar".to_string()]);
    }

    #[test]
    fn parse_args_double_dash_stops_option_parsing() {
        let opts = parse_args(vec![
            "-d".to_string(),
            "--".to_string(),
            "-weird".to_string(),
            "name".to_string(),
        ])
        .unwrap();
        assert!(opts.directory_only);
        assert_eq!(opts.list, vec!["-weird".to_string(), "name".to_string()]);
    }

    #[test]
    fn parse_args_rejects_unknown_option() {
        let err = parse_args(vec!["-x".to_string()]).unwrap_err();
        assert_eq!(err, "unknown option: -x");
    }
}
