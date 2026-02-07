# reru

**reru** is a high-performance, drop-in replacement for Python's `re` module, powered by Rust.

It combines the raw speed of Rust's linear-time regex engine with the flexibility of backtracking engines when necessary. `reru` intelligently analyzes your regex pattern and automatically selects the most efficient engine for the job.

## 🚀 Features

* **Multi-Stage Hybrid Architecture**:
    * **Tier 1 (Fastest)**: Uses the `regex` crate (linear time `O(n)`) for standard patterns, ensuring protection against ReDoS.
    * **Tier 2 (High Performance)**: Automatically switches to `pcre2` (JIT-compiled) for patterns with look-arounds (`(?=...)`) or backreferences. It is significantly faster than standard backtracking engines.
    * **Tier 3 (Fallback)**: Falls back to `fancy-regex` only for complex patterns not supported by the previous engines.
* **Global Caching**: Compilations are cached efficiently using a thread-safe `DashMap`, making repeated calls lightning fast across threads.
* **High Performance**: Implemented purely in Rust using `pyo3` and `maturin`.
* **Rich API**: Supports standard methods like `match`, `search`, `findall`, and `sub`, plus named capture groups.
* **Type Safe**: Includes full type hints (`.pyi`) for better IDE integration and static analysis.
* **Cross-Platform**: Pre-built wheels available for Linux (x86_64, aarch64, armv7, musl), macOS (Intel & Apple Silicon), and Windows (x64, x86, arm64).

## 📦 Installation

Install `reru` easily via pip:

```bash
pip install reru

```

## 🛠 Usage

`reru` exposes a simple API similar to Python's standard `re` module. Functions are available directly at the module level for optimized dispatch.

### Basic Matching and Searching

```python
import reru

# Check if a pattern matches (returns bool)
if reru.is_match(r"\d+", "The answer is 42"):
    print("It's a match!")

# Search for a pattern (returns a Match object or None)
match = reru.search(r"(\w+) world", "hello world")
if match:
    print(f"Full match: {match.group()}") # "hello world"
    print(f"Start index: {match.start()}") # 0
    print(f"End index: {match.end()}")     # 11

# Optimized usage
RE1 = reru.compile(r"\d+")
RE1.is_match("The answer is 42")
RE2 = reru.compile(r"(\w+) world")
RE2.search("hello world")
```

### Named Groups and New Methods
`reru` now supports named capture groups, `findall`, and `sub` (substitution).

```python
import reru

# Named Groups
match = reru.match(r"(?P<year>\d{4})-(?P<month>\d{2})", "2024-05")
if match:
    print(match.group("year"))  # "2024"
    print(match.group(1))       # "2024"

# Find All Matches
results = reru.findall(r"\d+", "Items: 10, 20, 30")
print(results) # ['10', '20', '30']

# Substitution
text = reru.sub(r"ERROR", "CRITICAL", "System status: ERROR")
print(text) # "System status: CRITICAL"
```



### ⚠ Important: Substitution Syntax Difference ($1 vs \1)

Unlike Python's re module which uses \1 or \g<name> for backreferences in substitutions, `reru` passes the replacement string directly to the underlying Rust engines.

<b>You must use Rust/PCRE syntax for substitutions:</b>

- Use $1, $2 instead of \1, \2.

- Use ${name} instead of \g<name>.

```python
import reru
import re

# ❌ Python Standard Syntax (Doesn't work in reru)
# re.sub(r"(\d+)", r"Value: \1", "100") 

# ✅ reru Syntax (Rust Style)
reru.sub(r"(\d+)", r"Value: $1", "100")
# Output: "Value: 100"

# Named Groups
reru.sub(r"(?P<val>\d+)", r"Value: ${val}", "100")
```

### Advanced Configuration
You can fine-tune the regex engine using `ReConfig`. This allows you to control case sensitivity, multiline modes, whitespace ignoring, and execution limits.

```python
from reru import ReConfig

config = ReConfig(case_insensitive=True, multiline=True)
match = reru.search(r"hello", "HELLO world", config=config)
```

### Engine Selection (Advanced)
If you need to force a specific engine (ignoring the auto-detection), you can use `compile_custom`:
```
from reru import SelectEngine, compile_custom

# Force the Standard Rust engine (strictly linear time)
pat = compile_custom(r"\d+", select_engine=SelectEngine.Std)

# Force Fancy engine (for full Python compatibility in substitution)
pat_fancy = compile_custom(r"\d+", select_engine=SelectEngine.Fancy)
```


## ⚙️ How It Works

reru uses a "Try-Fail" fallback strategy to ensure the best balance between performance and compatibility:

1. Stage 1 (Rust Regex): Attempts to compile with the `regex` crate. It guarantees linear time execution but doesn't support look-arounds or backreferences.

2. Stage 2 (PCRE2): If Stage 1 fails, it attempts to compile with `pcre2`. This is a high-performance JIT-compiled engine that supports most complex features (look-arounds, etc.).

3. Stage 3 (Fancy Regex): If PCRE2 fails or is unsuitable, it falls back to `fancy-regex` as a last resort to maintain maximum compatibility.



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