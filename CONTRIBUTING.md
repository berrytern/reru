# Contributing to reru

First off, thank you for considering contributing to `reru`! 🚀 

`reru` is an open-source project, and community contributions are incredibly valuable whether it's fixing bugs, improving documentation, adding new features, or optimizing performance.

## 🧠 Project Philosophy

`reru` aims to be a high-performance, drop-in replacement for Python's standard `re` module. When contributing, keep the following goals in mind:
1. **Compatibility:** `reru` should match the behavior of Python's `re` module as closely as possible.
2. **Performance:** Changes should maintain or improve the execution speed.
3. **Safety:** The multi-stage Rust engines (regex, pcre2, fancy-regex) should safely handle complex patterns without panicking or crashing the Python interpreter.

## 🛠️ Development Setup

To work on `reru`, you will need **Rust**, **Python (>=3.9)**, and **uv** installed on your system.

### 1. Clone the repository
```bash
git clone [https://github.com/berrytern/reru.git](https://github.com/berrytern/reru.git)
cd reru
```

2. Set up the Python environment

We use uv for fast dependency management.
Bash

uv venv
source .venv/bin/activate  # On Windows, use: .venv\Scripts\activate
uv sync

3. Build the Rust extension

reru uses maturin and PyO3 to bridge Rust and Python. To compile the Rust code and install it in your current virtual environment, run:
Bash

maturin develop --release

(Note: Omitting --release will build the debug version, which is faster to compile but much slower at runtime.)
## 🧪 Testing

Because `reru` is a drop-in replacement, ensuring compatibility with the standard library is our highest priority. We have ported tests from the official Python `re` test suite.

We use pytest for running our test suite. To run the tests:

``bash
pytest tests/
``bash

Adding Tests:
- If you are adding a new feature, please add corresponding tests in the tests/ directory.

- If you are fixing a bug, please add a regression test to ensure the bug does not reappear.

## 📊 Benchmarking

Performance is a key feature of reru. If your PR touches the core engine routing or matching logic, please run the benchmarks to ensure there are no performance regressions.

```bash
python benchmarks/benchmark.py
```

Include the benchmark results in your Pull Request description if you are making performance optimizations!
## 📝 Submitting a Pull Request

1. **Fork** the repository.
2. **Clone** your fork and checkout the `develop` branch:
   `git checkout develop`
3. **Create your feature branch** from `develop`:
   `git checkout -b feature/my-new-feature`
4. **Commit** your changes with clear, descriptive commit messages.
5. **Test** your changes thoroughly using `pytest`.
6. **Push** your branch to your fork.
7. **Open a Pull Request**! Ensure your PR targets the `develop` branch of the main `reru` repository. Describe the changes you made, the reasoning behind them, and link any relevant issues.

## 🐛 Reporting Issues

If you find a bug or have a feature request, please open an issue on GitHub.

When reporting a bug, please include:

- A minimal, reproducible Python script showing the issue.
- The expected output (usually what the standard re module outputs).
- The actual output from reru.
- Your OS, Python version, and reru version.

Thank you for helping make Python regex faster! 🦀🐍