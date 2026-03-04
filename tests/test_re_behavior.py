import pytest
import reru

# Test cases for simple regex patterns
def test_simple_regex():
    # Test '.' (any character except newline)
    assert reru.match("a.b", "acb") is not None
    assert reru.match("a.b", """a
c""") is None
    assert reru.match("a.b", "ab") is None

    # Test '*' (zero or more occurrences)
    assert reru.match("a*b", "b") is not None
    assert reru.match("a*b", "ab") is not None
    assert reru.match("a*b", "aaab") is not None

    # Test '+' (one or more occurrences)
    assert reru.match("a+b", "b") is None
    assert reru.match("a+b", "ab") is not None
    assert reru.match("a+b", "aaab") is not None

    # Test '?' (zero or one occurrence)
    assert reru.match("a?b", "b") is not None
    assert reru.match("a?b", "ab") is not None
    assert reru.match("a?b", "aab") is None

    # Test '[]' (character set)
    assert reru.match("a[cb]d", "acd") is not None
    assert reru.match("a[cb]d", "abd") is not None
    assert reru.match("a[cb]d", "add") is None

    # Test '|' (alternation)
    assert reru.match("a|b", "a") is not None
    assert reru.match("a|b", "b") is not None
    assert reru.match("a|b", "c") is None

    # Test '^' (start of string)
    assert reru.match("^a", "a") is not None
    assert reru.match("^a", "ba") is None

    # Test '$' (end of string)
    assert reru.match("a$", "a") is not None
    assert reru.match("a$", "ab") is None

# Test cases for named groups
def test_named_groups():
    m = reru.match(r"(?P<first>\w+)-(?P<second>\w+)", "hello-world")
    assert m is not None
    assert m.group("first") == "hello"
    assert m.group("second") == "world"
    assert m.groups() == ("hello", "world")


    m = reru.match(r"(?P<digit>\d+)", "123")
    assert m is not None
    assert m.group("digit") == "123"

    with pytest.raises(ValueError):
        reru.match(r"(?P<name>a)", "a").group("nonexistent")

# Test cases for backtracking
def test_backtracking():
    # Simple backtracking example
    m = reru.match("a+a", "aaa")
    assert m is not None
    assert m.group(0) == "aaa"

    # More complex backtracking with greedy quantifiers
    m = reru.match(r"(a+)(a*)", "aaaaa")
    assert m is not None
    assert m.groups() == ("aaaaa", "") # (a+) consumes all 'a's, leaving none for (a*)

    m = reru.match(r"(a*)(a+)", "aaaaa")
    assert m is not None
    assert m.groups() == ("aaaa", "a")

    # Backtracking with optional groups
    m = reru.match("a(b?)c", "ac")
    assert m is not None
    assert m.group(1) == ""

    m = reru.match("a(b?)c", "abc")
    assert m is not None
    assert m.group(1) == "b"

    # Backtracking with character classes and quantifiers
    m = reru.match(r"(\d+)(\s*)(\d+)", "123   456")
    assert m is not None
    assert m.groups() == ("123", "   ", "456")

    # Pathological case for some regex engines, but should be handled by a good one
    # This pattern `(a+)+b` applied to `aaaaab`
    # (a+)+ will try to match 'aaaaa', then one more 'a' not possible, then
    # it backtracks and (a+) tries to match 'aaaa', then (a+) tries to match 'a', etc.
    # until it gives up on the last 'a'
    m = reru.match(r"(a+)+b", "aaaaab")
    assert m is not None
    assert m.group(0) == "aaaaab"
    assert m.groups() == ("aaaaa",) # The last (a+) group captured 'a'