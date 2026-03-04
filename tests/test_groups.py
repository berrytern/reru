import pytest
import reru


def test_match_groups():
    assert reru.match("a", "a").groups() == ()
    assert reru.match("(a)", "a").groups() == ("a",)

    for a in ("\xe0", "\u0430", "\U0001d49c"):
        assert reru.match(a, a).groups() == ()
        assert reru.match("(%s)" % a, a).groups() == (a,)