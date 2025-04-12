# sample

`sample` is a fast, reliable command-line tool to randomly sample
lines from a file or standard input using [reservoir
sampling](https://en.wikipedia.org/wiki/Reservoir_sampling). It
samples without replacement.

Good for:
- Downsampling large datasets
- Sampling logs for debugging
- Creating reproducible random subsets of data

## 📦 Installation

If you have Rust installed, you can install `sample` with:

```bash
cargo install sample
```

Or build from source:

```bash
git clone https://github.com/stringertheory/sample.git
cd sample
cargo build --release
```

## 🚀 Usage

```bash
sample -n <NUM> [--seed <SEED>] [FILE]
```

Here are a few examples:

```bash
sample --help
cat data.txt | sample -n 10
sample -n 10 data.txt
sample -n 10 < data.txt
sample -n 10 --seed 17 < data.txt
```

### Options

| Option        | Description                             |
|---------------|-----------------------------------------|
| `-n <NUM>`     | Number of lines to sample (**required**) |
| `--seed <SEED>`| Optional seed for reproducible sampling |
| `-h`, `--help` | Show help message                       |

## 🧪 Testing

This project includes unit and integration tests:

```bash
cargo test
```

## 📝 License

Licensed under the [MIT License](LICENSE).

## 🤝 Contributing

Issues and pull requests welcome! If you have an idea, a feature
request, or a bug report, feel free to open an issue or PR.
