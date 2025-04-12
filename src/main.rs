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
}

/// Perform reservoir sampling on lines from a reader
///
/// # Arguments
///
/// * `reader` - A source of lines to sample from
/// * `k` - Number of lines to sample
/// * `rng` - Random number generator to use
///
/// # Returns
///
/// A vector containing the sampled lines
fn reservoir_sample<R: BufRead>(
    mut reader: R,
    k: usize,
    mut rng: StdRng,
) -> io::Result<Vec<String>> {
    let mut buf = String::new();
    let mut reservoir: Vec<String> = Vec::with_capacity(k);
    let mut total = 0;

    while reader.read_line(&mut buf)? > 0 {
        let line = buf.trim_end().to_string();
        if total < k {
            reservoir.push(line);
        } else {
            let j = rng.gen_range(0..=total);
            if j < k {
                reservoir[j] = line;
            }
        }
        total += 1;
        buf.clear();
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

    Ok(())
}

/// Parse command line arguments using clap
fn parse_args() -> Config {
    let matches = Command::new("samp")
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

    Config {
        sample_size: *matches.get_one::<usize>("sample_size").unwrap(),
        seed: matches.get_one::<u64>("seed").copied(),
        filename: matches.get_one::<String>("file").cloned(),
    }
}

fn main() -> io::Result<()> {
    let config = parse_args();

    // Set up the input source
    let reader: Box<dyn BufRead> = match &config.filename {
        Some(file) => {
            // Instead of using map_err with process::exit
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

    // Initialize RNG
    let rng = match config.seed {
        Some(s) => StdRng::seed_from_u64(s),
        None => StdRng::from_entropy(),
    };

    // Perform the sampling
    let samples = reservoir_sample(reader, config.sample_size, rng)?;

    // Output the results
    write_results(samples)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::io::Cursor;
    use std::io::Read;
    use std::io::Write;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use tempfile::NamedTempFile;

    // Helper function to find the executable path
    fn find_executable() -> PathBuf {
        // Try different common locations for the binary
        let possible_locations = [
            // Regular debug build location
            PathBuf::from("target/debug/samp"),
            // Release build location
            PathBuf::from("target/release/samp"),
            // Current directory with bin extension (Windows)
            PathBuf::from("samp.exe"),
            // Current directory
            PathBuf::from("samp"),
        ];

        for path in &possible_locations {
            if path.exists() {
                return path.clone();
            }
        }

        // If we can't find it, use the first option and let the test fail with a clear error
        eprintln!("Warning: Executable not found in common locations. Tests might fail.");
        possible_locations[0].clone()
    }

    // Unit tests for core algorithm
    #[test]
    fn test_reservoir_sampling_properties() {
        let input_data = "a\nb\nc\nd\ne\nf\ng\nh\ni\nj\n";
        let k = 5;
        let seed = 12345;

        let run_sample = || {
            let reader = Cursor::new(input_data);
            let rng = StdRng::seed_from_u64(seed);
            reservoir_sample(reader, k, rng).unwrap()
        };

        // Run the sampler twice with the same seed
        let sample1 = run_sample();
        let sample2 = run_sample();

        // Check: correct sample size
        assert_eq!(sample1.len(), k);
        assert_eq!(sample2.len(), k);

        // Check: all items are from the original data
        let input_set: HashSet<_> = input_data.lines().collect();
        for item in &sample1 {
            assert!(input_set.contains(&item.as_str()));
        }

        // Check: samples match with same seed
        assert_eq!(sample1, sample2);
    }

    #[test]
    fn test_k_greater_than_input_len() {
        let input_data = "a\nb\nc\nd\n";
        let k = 6; // Greater than the input length
        let seed = 42;

        let reader = Cursor::new(input_data);
        let rng = StdRng::seed_from_u64(seed);
        let sample = reservoir_sample(reader, k, rng).unwrap();

        // Ensure we only get the available lines (input_len is 4, we asked for 6)
        let input_lines: Vec<_> = input_data.lines().collect();
        assert_eq!(sample.len(), input_lines.len());

        // Check that all input lines are in the sample
        let sample_set: HashSet<_> = sample.iter().collect();
        for line in input_lines {
            assert!(sample_set.contains(&line.to_string()));
        }
    }

    // Integration tests using actual processes
    #[test]
    fn test_stdin_behavior() {
        let input_data = "a\nb\nc\nd\ne\n";
        let expected_sample_size = 3;

        // Get the path to the executable
        let exe_path = find_executable();

        // Create the Command with piped stdin
        let mut child = Command::new(&exe_path)
            .arg("-n")
            .arg(expected_sample_size.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to execute process");

        // Write to stdin
        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        stdin.write_all(input_data.as_bytes()).unwrap();
        drop(stdin); // Close stdin to signal end of input

        // Wait for the command to finish and capture output
        let output = child.wait_with_output().expect("Failed to wait on child");

        // Capture stdout as a string
        let result = String::from_utf8_lossy(&output.stdout);

        // Verify that the output is the correct length
        let result_lines: Vec<&str> = result.lines().collect();
        assert_eq!(result_lines.len(), expected_sample_size);

        // Ensure all sampled lines came from the input
        for line in result_lines {
            assert!(input_data.contains(line));
        }
    }

    #[test]
    fn test_pipeline_behavior() {
        let input_data = "a\nb\nc\nd\ne\n";
        let sample_size = 4;

        // Get the path to the executable
        let exe_path = find_executable();

        // First setup - run without head
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

        // Second setup - pipe through head to test broken pipe handling
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

        // Read just the first part of stdout to simulate a broken pipe
        let mut stdout = child_with_head
            .stdout
            .take()
            .expect("Failed to open stdout");
        let mut buf = [0u8; 10]; // Read just a few bytes
        stdout.read(&mut buf).expect("Failed to read from stdout");
        drop(stdout); // Close stdout to simulate broken pipe

        // The program should exit gracefully with status 0 when pipe is broken
        let status = child_with_head.wait().expect("Failed to wait on child");
        assert!(status.success());
    }

    #[test]
    fn test_file_input() {
        // Get the path to the executable
        let exe_path = find_executable();

        // Create a temporary file with test data
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let input_data = "line1\nline2\nline3\nline4\nline5\n";
        temp_file
            .write_all(input_data.as_bytes())
            .expect("Failed to write to temp file");
        let temp_path = temp_file.path().to_str().unwrap();

        // Run samp on the file
        let output = Command::new(&exe_path)
            .arg("-n")
            .arg("3")
            .arg("--seed")
            .arg("42") // For deterministic output
            .arg(temp_path)
            .output()
            .expect("Failed to execute process");

        let result = String::from_utf8_lossy(&output.stdout);
        let result_lines: Vec<&str> = result.lines().collect();

        // Check that we got exactly 3 lines
        assert_eq!(result_lines.len(), 3);

        // Check that all lines are from the input file
        for line in result_lines {
            assert!(input_data.contains(line));
        }
    }
}
