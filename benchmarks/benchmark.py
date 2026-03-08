#!/usr/bin/env python3
import reru
import timeit
import matplotlib.pyplot as plt

benchmarks = [
    ('Python|Perl', 'Perl'),
    ('(Python|Perl)', 'Perl'),
    ('Python|Perl|Tcl', 'Perl'),
    ('(Python|Perl|Tcl)', 'Perl'),
    ('(Python)\\1', 'PythonPython'),
    ('([0a-z][a-z0-9]*,)+', 'a5,b7,c9,'),
    ('([a-z][a-z0-9]*,)+', 'a5,b7,c9,'),
    ('Python', 'Python'),
    ('.*Python', 'Python'),
    ('.*Python.*', 'Python'),
    ('.*(Python)', 'Python'),
]

def run_benchmarks_and_plot():
    # Create a large padding string to test search performance on larger texts
    noise_levels = [5, 25, 50, 250, 500, 1000]
    padding = ["abcdefghijklmnopqrstuvwxyz" * n for n in noise_levels]
    iterations = 10_000

    # Dictionary to store speedup results for plotting
    results = {
        pattern: {"sizes": [], "speedups": []}
        for pattern, _ in benchmarks
    }

    print(f"Running benchmarks ({iterations} iterations each)...\n")

    for noise, pad in zip(noise_levels, padding):
        noise_chars = noise * 26 # Padding is added to both sides
        print(f"Testing with noise padding: {noise_chars} characters...")

        for pattern, target in benchmarks:
            text = pad + target + pad
            total_size = len(text)
            
            # Test Python's standard re
            re_time = timeit.timeit(
                stmt=f"re.search({pattern!r}, text)",
                setup=f"import re; text = {text!r}; re.search({pattern!r}, text)",
                number=iterations
            )
            
            # Test reru
            reru_time = timeit.timeit(
                stmt=f"reru.is_search({pattern!r}, text)",
                setup=f"import reru; text = {text!r}; reru.is_search({pattern!r}, text)",
                number=iterations
            )
            
            # Calculate speedup
            speedup = re_time / reru_time if reru_time > 0 else 0
            
            # Store the data
            results[pattern]["sizes"].append(total_size)
            results[pattern]["speedups"].append(speedup)

    print("Benchmarking complete. Generating graph...")
    plot_results(results)

def plot_results(results):
    plt.figure(figsize=(12, 8)) # Set a good size for the single graph

    for pattern, data in results.items():
        # Truncate pattern for the legend so it doesn't take up too much space
        display_pat = pattern if len(pattern) <= 25 else pattern[:22] + "..."
        
        # Plot the line for this pattern
        plt.plot(data["sizes"], data["speedups"], marker='o', label=display_pat, linewidth=2.5)

    # Formatting the graph
    plt.title("Performance Gain: reru vs standard 're'", fontsize=18, fontweight='bold', pad=20)
    plt.xlabel("Input String Length (Characters)", fontsize=14)
    plt.ylabel("Speedup Multiplier (x times faster)", fontsize=14)
    
    # Add a horizontal line at 1.0 to show the baseline (where reru = re)
    plt.axhline(y=1.0, color='red', linestyle='--', alpha=0.5, label='Baseline (re speed)')
    
    plt.grid(True, linestyle='--', alpha=0.7)
    
    # Move the legend outside the graph so it doesn't cover the lines
    plt.legend(bbox_to_anchor=(1.02, 1), loc='upper left', fontsize=11, title="Patterns", title_fontsize=12)
    plt.tight_layout()
    
    # Save and show
    plt.savefig("benchmark.png", dpi=300, bbox_inches="tight")
    print("Graph saved as 'benchmark.png'")
    # plt.show()

if __name__ == '__main__':
    run_benchmarks_and_plot()