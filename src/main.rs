use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::process;
use std::io::ErrorKind;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut n = None;
    let mut seed: Option<u64> = None;
    let mut filename = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-n" => {
                if i + 1 < args.len() {
                    n = args[i + 1].parse::<usize>().ok();
                    i += 1;
                }
            }
            "--seed" => {
                if i + 1 < args.len() {
                    seed = args[i + 1].parse::<u64>().ok();
                    i += 1;
                }
            }
            "--help" | "-h" => {
                println!(
"Usage: sample -n <NUM> [--seed <SEED>] [FILE]
Randomly sample lines from a file or stdin using reservoir sampling.

Options:
  -n <NUM>       Number of lines to sample (required)
  --seed <SEED>  Optional seed for reproducible sampling
  -h, --help     Show this help message

Example usage:
    cat data.txt | sample -n 20   # Sample 20 lines from data.txt"
                );
                return;
            }
            _ => {
                filename = Some(args[i].clone());
            }
        }
        i += 1;
    }

    let k = n.unwrap_or_else(|| {
        eprintln!("Usage: sample -n <NUM> [--seed <SEED>] [FILE]");
        process::exit(1);
    });

    let mut input: Box<dyn BufRead> = match filename {
	Some(file) => {
            let f = File::open(file).unwrap_or_else(|_| {
		eprintln!("Error: cannot open input file.");
		process::exit(1);
            });
            Box::new(BufReader::new(f))
	}
	None => Box::new(BufReader::new(io::stdin())),
    };
    
    let mut rng: StdRng = match seed {
        Some(s) => SeedableRng::seed_from_u64(s),
        None => StdRng::from_entropy(),
    };

    let mut buf = String::new();
    let mut reservoir: Vec<String> = Vec::with_capacity(k);
    let mut total = 0;

    while input.read_line(&mut buf).unwrap_or(0) != 0 {
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

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    for line in reservoir {
        if let Err(e) = writeln!(handle, "{}", line) {
            if e.kind() == ErrorKind::BrokenPipe {
                process::exit(0);
            } else {
                eprintln!("Error writing to stdout: {}", e);
                process::exit(1);
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    

    #[test]
    fn test_reservoir_sampling_properties() {
	use std::collections::HashSet;

	let input_data = vec![
            "a", "b", "c", "d", "e", "f", "g", "h", "i", "j"
	];
	let k = 5;
	let seed = 12345;

	let run_sample = || {
            let mut rng: StdRng = SeedableRng::seed_from_u64(seed);
            let mut reservoir: Vec<&str> = Vec::with_capacity(k);

            for (i, &line) in input_data.iter().enumerate() {
		if i < k {
                    reservoir.push(line);
		} else {
                    let j = rng.gen_range(0..=i);
                    if j < k {
			reservoir[j] = line;
                    }
		}
            }

            reservoir
	};

	// Run the sampler twice with the same seed
	let sample1 = run_sample();
	let sample2 = run_sample();

	// Check: correct sample size
	assert_eq!(sample1.len(), k);
	assert_eq!(sample2.len(), k);

	// Check: all items are from the original data
	let input_set: HashSet<_> = input_data.iter().cloned().collect();
	for item in &sample1 {
            assert!(input_set.contains(item));
	}

	// Check: samples match with same seed
	assert_eq!(sample1, sample2);
    }

    #[test]
    fn test_k_greater_than_input_len() {
	let input_data = vec!["a", "b", "c", "d"];
	let k = 6; // Greater than the input length
	let seed = 42;

	let run_sample = || {
            let mut rng: StdRng = SeedableRng::seed_from_u64(seed);
            let mut reservoir: Vec<&str> = Vec::with_capacity(k);

            for (i, &line) in input_data.iter().enumerate() {
		if i < k {
                    reservoir.push(line);
		} else {
                    let j = rng.gen_range(0..=i);
                    if j < k {
			reservoir[j] = line;
                    }
		}
            }

            reservoir
	};

	// Run the sample and check the size
	let sample = run_sample();

	// Ensure we only get the available lines (input_len is 4, we asked for 6)
	assert_eq!(sample.len(), input_data.len());
	assert!(sample.contains(&"a"));
	assert!(sample.contains(&"b"));
	assert!(sample.contains(&"c"));
	assert!(sample.contains(&"d"));
    }

    #[test]
    fn test_stdin_behavior() {
	use std::process::{Command, Stdio};
	use std::io::Write;

	let input_data = "a\nb\nc\nd\ne\n";
	let expected_sample_size = 3;

	// Create the Command with piped stdin
	let mut child = Command::new("target/debug/sample")
            .arg("-n")
            .arg(expected_sample_size.to_string())  // Pass number of samples
            .stdin(Stdio::piped())  // We need to pipe the input
            .stdout(Stdio::piped()) // To capture the output
            .stderr(Stdio::piped()) // To capture stderr if needed
            .spawn()
            .expect("Failed to execute process");

	// Now we have access to the stdin of the child process
	let mut stdin = child.stdin.take().expect("Failed to open stdin");
	stdin.write_all(input_data.as_bytes()).unwrap();
	
	// Close stdin to signal end of input
	drop(stdin);

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
    

    
}
