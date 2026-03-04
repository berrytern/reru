#!/usr/bin/env python3
import timeit

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

def run_benchmarks():
    # Create a large padding string to test search performance on larger texts
    noise = [5, 50, 500]
    padding = ["abcdefghijklmnopqrstuvwxyz" * n for n in noise]
    iterations = 10_000

    for noise, pad in zip(noise, padding):
        print(f"Running benchmarks ({iterations} iterations each)...\n")
        print(f"Noise padding: {noise * 26} characters added to target strings")
        print(f"{'Pattern':<25} | {'re (sec)':<10} | {'reru (sec)':<10} | {'Speedup'}")
        print("-" * 65)

        for pattern, target in benchmarks:
            text = pad + target + pad
            
            # Test Python's standard re
            re_time = timeit.timeit(
                stmt=f"re.search({pattern!r}, text)",
                setup=f"import re; text = {text!r};re.search({pattern!r}, text)",
                number=iterations
            )
            
            # Test reru
            reru_time = timeit.timeit(
                stmt=f"reru.is_search({pattern!r}, text)",
                setup=f"import reru; text = {text!r}; reru.is_search({pattern!r}, text)",
                number=iterations
            )
            
            # Calculate speedup multiplier
            speedup = re_time / reru_time if reru_time > 0 else 0
            
            # Truncate pattern for display if it's too long
            display_pat = pattern if len(pattern) <= 23 else pattern[:20] + "..."
            print(f"{display_pat:<25} | {re_time:<10.4f} | {reru_time:<10.4f} | {speedup:.2f}x")

if __name__ == '__main__':
    run_benchmarks()