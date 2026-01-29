# reru

**reru** is a high-performance, drop-in replacement for Python's `re` module, powered by Rust.

It combines the raw speed of Rust's linear-time regex engine with the flexibility of backtracking engines when necessary. `reru` intelligently analyzes your regex pattern and automatically selects the most efficient engine for the job.

## 🚀 Features

* **Hybrid Engine Architecture**:
* **Fast Path**: Uses the `regex` crate (linear time `O(n)`) for standard patterns, ensuring protection against ReDoS (Regular Expression Denial of Service).
* **Feature Path**: Automatically switches to `fancy-regex` for patterns requiring look-arounds (`(?=...)`, `(?<=...)`) or backreferences (`\1`), maintaining compatibility with Python's standard regex features.


* **High Performance**: Implemented purely in Rust using `pyo3` and `maturin`.
* **Built-in Caching**: Compilations are cached efficiently using a concurrent `DashMap` implementation, making repeated calls lightning fast.
* **Type Safe**: Includes full type hints (`.pyi`) for better IDE integration and static analysis.
* **Cross-Platform**: Pre-built wheels available for Linux (x86_64, aarch64, armv7, musl), macOS (Intel & Apple Silicon), and Windows (x64, x86, arm64).

## 📦 Installation

Install `reru` easily via pip:

```bash
pip install reru

```

## 🛠 Usage

`reru` exposes a simple API similar to Python's standard `re` module but encapsulates methods within the `ReRu` class for optimized dispatch.

### Basic Matching and Searching

```python
from reru import ReRu

# Check if a pattern matches (returns bool)
if ReRu.is_match(r"\d+", "The answer is 42"):
    print("It's a match!")

# Search for a pattern (returns a Match object or None)
match = ReRu.search(r"(\w+) world", "hello world")
if match:
    print(f"Full match: {match.group()}") # "hello world"
    print(f"Start index: {match.start()}") # 0
    print(f"End index: {match.end()}")     # 11

```

### Advanced Configuration

You can fine-tune the regex engine using `ReConfig`. This allows you to control case sensitivity, multiline modes, whitespace ignoring, and execution limits.

```python
from reru import ReRu, ReConfig

# Configure the regex engine
config = ReConfig(
    case_insensitive=True,
    ignore_whitespace=False,
    multiline=True,
    unicode_mode=True,
    size_limit=10 * (1 << 20),  # 10 MB size limit
    dfa_size_limit=2 * (1 << 20), # 2 MB DFA limit
    backtrack_limit=1_000_000     # Limit for backtracking engine
)

# Perform search with config
match = ReRu.search(r"ERROR", "Critical error occurred", config=config)
if match:
    print("Found error!")

```

## ⚙️ How It Works

Under the hood, `reru` inspects the byte-code of your pattern before compilation:

1. **Inspection**: It checks for "expensive" features like look-aheads, look-behinds, and backreferences.
2. **Selection**:
* If expensive features are **absent**, it uses Rust's `regex` crate. This guarantees linear time execution and is generally faster.
* If expensive features are **present**, it falls back to `fancy-regex`. This supports the complex features Python developers expect while still benefiting from Rust's optimizations where possible.



## 💻 Development

To build `reru` from source, you will need **Rust** and **uv** (or `maturin`) installed.

1. **Clone the repository**:
```bash
git clone https://github.com/berrytern/reru.git
cd reru

```


2. **Setup environment**:
```bash
uv venv
source .venv/bin/activate
uv sync

```


3. **Build and install**:
```bash
maturin develop --release

```


## 📄 License

This project is licensed under the Apache License 2.0. See the [LICENSE](https://www.apache.org/licenses/LICENSE-2.0.txt) file for details.

## 👥 Authors

* **João Pedro Miranda C. Hluchan** - *Initial work* - [berrytern](https://github.com/berrytern)