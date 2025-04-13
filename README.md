# sample-lines

`samp` is a fast command-line tool to randomly sample
lines from a file or standard input using [reservoir
sampling](https://en.wikipedia.org/wiki/Reservoir_sampling). It
samples uniformly without replacement.

Good for:
- Downsampling large datasets
- Sampling logs for debugging
- Creating reproducible random subsets of data

You can think of `samp` kind of like `head` or `tail`, for example:

```bash
head -n 10 < data.txt   # outputs 10 first lines
tail -n 10 < data.txt   # outputs 10 last lines
samp -n 10 < data.txt   # outputs 10 random lines
```

## Installation

If you have Rust installed, you can install `samp` with:

```bash
cargo install sample-lines
```

Or build it from source:

```bash
git clone https://github.com/stringertheory/sample-lines.git
cd sample-lines
cargo build --release
```

## Usage

```bash
samp -n <NUM> [--seed <SEED>] [FILE]
```

Here are a few examples:

```bash
samp --help                                   # Show help
cat data.txt | samp -n 10                     # Keep 10 lines, with pipe
samp -n 10 data.txt                           # Giving filename
samp -n 10 < data.txt                         # Standard in
samp -n 10 --seed 17 < data.txt               # Reproducible sample
cat data.csv | samp -n 10 --preserve-headers  # Preserve 1 header line
samp -r 0.01 < big.log                        # Keep ~1% of lines
samp -r 0.10 --seed 17 data.csv -p            # Reproducible 10% sample
```

### Options

| Option | Description |
|--------|-------------|
| `-n, --number <NUM>` | Number of lines to sample (**required**) |
| `-r, --rate <RATE>` | Sampling rate: probability to include each line (e.g., 0.05) |
| `-s, --seed <SEED>` | Optional seed for reproducible sampling |
| `-p, --preserve-headers [N]` | Preserve the first `N` lines as headers (default: 1 if flag is used) |
| `-h`, `--help` | Show help message |
| `--version` | Show the version number |

## Testing

```bash
cargo clean
cargo build # need binary for testing stdin/stderr
cargo test
```

## License

Licensed under the [MIT License](LICENSE).

## Contributing

Issues and pull requests welcome! If you have an idea, a feature
request, or a bug report, feel free to open an issue or PR.
