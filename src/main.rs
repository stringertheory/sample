use clap::{Arg, Command};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::fs::File;
use std::io::ErrorKind;
use std::io::{self, BufRead, BufReader, Write};
use std::process;

/// Configuration for the sampling program
struct Config {
    sample_size: usize,
    seed: Option<u64>,
    filename: Option<String>,
    preserve_headers: Option<usize>,
}

/// Perform reservoir sampling on lines from an iterator
fn reservoir_sample<I>(lines: I, k: usize, mut rng: StdRng) -> io::Result<Vec<String>>
where
    I: Iterator<Item = Result<String, io::Error>>,
{
    let mut reservoir: Vec<String> = Vec::with_capacity(k);

    for (total, line_result) in lines.enumerate() {
        let line = line_result?;
        if total < k {
            reservoir.push(line);
        } else {
            let j = rng.gen_range(0..=total);
            if j < k {
                reservoir[j] = line;
            }
        }
    }

    Ok(reservoir)
}

/// Write sampled lines to stdout, handling broken pipes gracefully
fn write_results(lines: Vec<String>) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    for line in lines {
        match writeln!(handle, "{}", line) {
            Ok(_) => (),
            Err(e) if e.kind() == ErrorKind::BrokenPipe => process::exit(0),
            Err(e) => return Err(e),
        }
    }

    handle.flush()?;
    Ok(())
}

/// Parse command line arguments using clap
fn parse_args() -> Config {
    let matches = Command::new("samp")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Randomly sample lines from a file or stdin using reservoir sampling")
        .arg(
            Arg::new("sample_size")
                .short('n')
                .value_name("NUM")
                .help("Number of lines to sample (required)")
                .required(true)
                .value_parser(clap::value_parser!(usize)),
        )
        .arg(
            Arg::new("seed")
                .short('s')
                .long("seed")
                .value_name("SEED")
                .help("Optional seed for reproducible sampling")
                .value_parser(clap::value_parser!(u64)),
        )
        .arg(
            Arg::new("preserve_headers")
                .short('p')
                .long("preserve-headers")
                .num_args(0..=1)
                .value_name("NUM")
                .help(
                    "Number of header lines to preserve (default: 1 if specified without a value)",
                )
                .value_parser(clap::value_parser!(usize)),
        )
        .arg(
            Arg::new("file")
                .value_name("FILE")
                .help("Input file (reads from stdin if not provided)")
                .index(1),
        )
        .after_help(
            "Example usage:
    cat data.txt | samp -n 20   # Sample 20 lines from data.txt",
        )
        .get_matches();

    let preserve_headers = if matches.contains_id("preserve_headers") {
        Some(
            matches
                .get_one::<usize>("preserve_headers")
                .copied()
                .unwrap_or(1),
        )
    } else {
        None
    };

    Config {
        sample_size: *matches.get_one::<usize>("sample_size").unwrap(),
        seed: matches.get_one::<u64>("seed").copied(),
        filename: matches.get_one::<String>("file").cloned(),
        preserve_headers,
    }
}

fn main() -> io::Result<()> {
    let config = parse_args();

    // Set up the input source
    let reader: Box<dyn BufRead> = match &config.filename {
        Some(file) => {
            let f = match File::open(file) {
                Ok(file) => file,
                Err(e) => {
                    eprintln!("Error: cannot open input file: {}", e);
                    process::exit(1);
                }
            };
            Box::new(BufReader::new(f))
        }
        None => Box::new(BufReader::new(io::stdin())),
    };

    let mut lines = reader.lines();

    // Handle preserved headers
    if let Some(num_headers) = config.preserve_headers {
        for _ in 0..num_headers {
            match lines.next() {
                Some(Ok(line)) => println!("{}", line),
                Some(Err(e)) => {
                    eprintln!("Error reading input: {}", e);
                    process::exit(1);
                }
                None => return Ok(()),
            }
        }
    }

    let rng = match config.seed {
        Some(s) => StdRng::seed_from_u64(s),
        None => StdRng::from_entropy(),
    };

    let samples = reservoir_sample(lines, config.sample_size, rng)?;
    write_results(samples)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::io::{Cursor, Read, Write};
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use tempfile::NamedTempFile;

    fn find_executable() -> PathBuf {
        let possible_locations = [
            PathBuf::from("target/debug/samp"),
            PathBuf::from("target/release/samp"),
            PathBuf::from("samp.exe"),
            PathBuf::from("samp"),
        ];

        for path in &possible_locations {
            if path.exists() {
                return path.clone();
            }
        }

        eprintln!("Warning: Executable not found in common locations. Tests might fail.");
        possible_locations[0].clone()
    }

    #[test]
    fn test_reservoir_sampling_properties() {
        let input_data = "a\nb\nc\nd\ne\nf\ng\nh\ni\nj\n";
        let k = 5;
        let seed = 12345;

        let run_sample = || {
            let reader = Cursor::new(input_data);
            let rng = StdRng::seed_from_u64(seed);
            reservoir_sample(reader.lines(), k, rng).unwrap()
        };

        let sample1 = run_sample();
        let sample2 = run_sample();

        assert_eq!(sample1.len(), k);
        assert_eq!(sample2.len(), k);

        let input_set: HashSet<_> = input_data.lines().collect();
        for item in &sample1 {
            assert!(input_set.contains(&item.as_str()));
        }

        assert_eq!(sample1, sample2);
    }

    #[test]
    fn test_k_greater_than_input_len() {
        let input_data = "a\nb\nc\nd\n";
        let k = 6;
        let seed = 17;

        let reader = Cursor::new(input_data);
        let rng = StdRng::seed_from_u64(seed);
        let sample = reservoir_sample(reader.lines(), k, rng).unwrap();

        let input_lines: Vec<_> = input_data.lines().collect();
        assert_eq!(sample.len(), input_lines.len());

        let sample_set: HashSet<_> = sample.iter().collect();
        for line in input_lines {
            assert!(sample_set.contains(&line.to_string()));
        }
    }

    #[test]
    fn test_preserve_headers() {
        let input = "h1\nh2\na\nb\nc\nd\n";
        let exe_path = find_executable();

        let output = Command::new(&exe_path)
            .arg("-n")
            .arg("2")
            .arg("-p")
            .arg("2")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                child.stdin.as_mut().unwrap().write_all(input.as_bytes())?;
                child.wait_with_output()
            })
            .expect("Failed to run samp");

        let result = String::from_utf8_lossy(&output.stdout);
        let mut lines = result.lines();

        assert_eq!(lines.next(), Some("h1"));
        assert_eq!(lines.next(), Some("h2"));

        let sampled: Vec<&str> = lines.collect();
        assert_eq!(sampled.len(), 2);
        for &line in &sampled {
            assert!(input.contains(line));
        }
    }

    #[test]
    fn test_stdin_behavior() {
        let input_data = "a\nb\nc\nd\ne\n";
        let expected_sample_size = 3;
        let exe_path = find_executable();

        let mut child = Command::new(&exe_path)
            .arg("-n")
            .arg(expected_sample_size.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute process");

        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        stdin.write_all(input_data.as_bytes()).unwrap();
        drop(stdin);

        let output = child.wait_with_output().expect("Failed to wait on child");
        let result = String::from_utf8_lossy(&output.stdout);
        let result_lines: Vec<&str> = result.lines().collect();

        assert_eq!(result_lines.len(), expected_sample_size);
        for line in result_lines {
            assert!(input_data.contains(line));
        }
    }

    #[test]
    fn test_pipeline_behavior() {
        let input_data = "a\nb\nc\nd\ne\n";
        let sample_size = 4;
        let exe_path = find_executable();

        let mut child_normal = Command::new(&exe_path)
            .arg("-n")
            .arg(sample_size.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute process");

        {
            let mut stdin = child_normal.stdin.take().expect("Failed to open stdin");
            stdin.write_all(input_data.as_bytes()).unwrap();
        }

        let normal_output = child_normal
            .wait_with_output()
            .expect("Failed to wait on child");
        let normal_result = String::from_utf8_lossy(&normal_output.stdout);
        let normal_lines = normal_result.lines().count();
        assert_eq!(normal_lines, sample_size);

        let mut child_with_head = Command::new(&exe_path)
            .arg("-n")
            .arg(sample_size.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute process");

        {
            let mut stdin = child_with_head.stdin.take().expect("Failed to open stdin");
            stdin.write_all(input_data.as_bytes()).unwrap();
        }

        let mut stdout = child_with_head
            .stdout
            .take()
            .expect("Failed to open stdout");
        let mut buf = [0u8; 10];
        stdout.read(&mut buf).expect("Failed to read from stdout");
        drop(stdout);

        let status = child_with_head.wait().expect("Failed to wait on child");
        assert!(status.success());
    }

    #[test]
    fn test_file_input() {
        let exe_path = find_executable();
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let input_data = "line1\nline2\nline3\nline4\nline5\n";
        temp_file
            .write_all(input_data.as_bytes())
            .expect("Failed to write to temp file");
        let temp_path = temp_file.path().to_str().unwrap();

        let output = Command::new(&exe_path)
            .arg("-n")
            .arg("3")
            .arg("--seed")
            .arg("17")
            .arg(temp_path)
            .output()
            .expect("Failed to execute process");

        let result = String::from_utf8_lossy(&output.stdout);
        let result_lines: Vec<&str> = result.lines().collect();

        assert_eq!(result_lines.len(), 3);
        for line in result_lines {
            assert!(input_data.contains(line));
        }
    }
}
