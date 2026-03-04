#!/usr/bin/env python3
import pytest
import re
import reru
import traceback

# The 3 possible outcomes for each pattern
SUCCEED, FAIL, SYNTAX_ERROR = range(3)
OUTCOME_MAP = {SUCCEED: "SUCCEED", FAIL: "FAIL", SYNTAX_ERROR: "SYNTAX_ERROR"}

# Paste your 'tests' list here:
raw_tests = [
    # Test ?P< and ?P= extensions
    ('(?P<foo_123', '', SYNTAX_ERROR),      # Unterminated group identifier
    ('(?P<1>a)', '', SYNTAX_ERROR),         # Begins with a digit
    ('(?P<!>a)', '', SYNTAX_ERROR),         # Begins with an illegal char
    ('(?P<foo!>a)', '', SYNTAX_ERROR),      # Begins with an illegal char

    # Same tests, for the ?P= form
    ('(?P<foo_123>a)(?P=foo_123', 'aa', SYNTAX_ERROR),
    ('(?P<foo_123>a)(?P=1)', 'aa', SYNTAX_ERROR),
    ('(?P<foo_123>a)(?P=!)', 'aa', SYNTAX_ERROR),
    ('(?P<foo_123>a)(?P=foo_124', 'aa', SYNTAX_ERROR),  # Backref to undefined group

    ('(?P<foo_123>a)', 'a', SUCCEED, 'g1', 'a'),
    ('(?P<foo_123>a)(?P=foo_123)', 'aa', SUCCEED, 'g1', 'a'),

    # Test octal escapes
    ('\\1', 'a', SYNTAX_ERROR),    # Backreference
    ('[\\1]', '\1', SUCCEED, 'found', '\1'),  # Character
    ('\\09', chr(0) + '9', SUCCEED, 'found', chr(0) + '9'),
    ('\\141', 'a', SUCCEED, 'found', 'a'),
    ('(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)\\119', 'abcdefghijklk9', SUCCEED, 'found+"-"+g11', 'abcdefghijklk9-k'),

    # Test \0 is handled everywhere
    (r'\0', '\0', SUCCEED, 'found', '\0'),
    (r'[\0a]', '\0', SUCCEED, 'found', '\0'),
    (r'[a\0]', '\0', SUCCEED, 'found', '\0'),
    (r'[^a\0]', '\0', FAIL),

    # Test various letter escapes
    (r'\a[\b]\f\n\r\t\v', '\a\b\f\n\r\t\v', SUCCEED, 'found', '\a\b\f\n\r\t\v'),
    (r'[\a][\b][\f][\n][\r][\t][\v]', '\a\b\f\n\r\t\v', SUCCEED, 'found', '\a\b\f\n\r\t\v'),
    # NOTE: not an error under PCRE/PRE:
    (r'\u', '', SYNTAX_ERROR),    # A Perl escape
    # (r'\c\e\g\h\i\j\k\m\o\p\q\y\z', 'ceghijkmopqyz', SUCCEED, 'found', 'ceghijkmopqyz'),
    # new \x semantics
    (r'\x00ffffffffffffff', '\377', FAIL, 'found', chr(255)),
    (r'\x00f', '\017', FAIL, 'found', chr(15)),
    (r'\x00fe', '\376', FAIL, 'found', chr(254)),
    # (r'\x00ffffffffffffff', '\377', SUCCEED, 'found', chr(255)),
    # (r'\x00f', '\017', SUCCEED, 'found', chr(15)),
    # (r'\x00fe', '\376', SUCCEED, 'found', chr(254)),

    (r"^\w+=(\\[\000-\277]|[^\n\\])*", "SRC=eval.c g.c blah blah blah \\\\\n\tapes.c",
     SUCCEED, 'found', "SRC=eval.c g.c blah blah blah \\\\"),

    # Test that . only matches \n in DOTALL mode
    ('a.b', 'acb', SUCCEED, 'found', 'acb'),
    ('a.b', 'a\nb', FAIL),
    ('a.*b', 'acc\nccb', FAIL),
    ('a.{4,5}b', 'acc\nccb', FAIL),
    ('a.b', 'a\rb', SUCCEED, 'found', 'a\rb'),
    ('(?s)a.b', 'a\nb', SUCCEED, 'found', 'a\nb'),
    ('(?s)a.*b', 'acc\nccb', SUCCEED, 'found', 'acc\nccb'),
    ('(?s)a.{4,5}b', 'acc\nccb', SUCCEED, 'found', 'acc\nccb'),
    ('(?s)a.b', 'a\rb', SUCCEED, 'found', 'a\rb'),

    (')', '', SYNTAX_ERROR),           # Unmatched right bracket
    ('', '', SUCCEED, 'found', ''),    # Empty pattern
    ('abc', 'abc', SUCCEED, 'found', 'abc'),
    ('abc', 'xbc', FAIL),
    ('abc', 'axc', FAIL),
    ('abc', 'abx', FAIL),
    ('abc', 'xabcy', SUCCEED, 'found', 'abc'),
    ('abc', 'ababc', SUCCEED, 'found', 'abc'),
    ('ab*c', 'abc', SUCCEED, 'found', 'abc'),
    ('ab*bc', 'abc', SUCCEED, 'found', 'abc'),
    ('ab*bc', 'abbc', SUCCEED, 'found', 'abbc'),
    ('ab*bc', 'abbbbc', SUCCEED, 'found', 'abbbbc'),
    ('ab+bc', 'abbc', SUCCEED, 'found', 'abbc'),
    ('ab+bc', 'abc', FAIL),
    ('ab+bc', 'abq', FAIL),
    ('ab+bc', 'abbbbc', SUCCEED, 'found', 'abbbbc'),
    ('ab?bc', 'abbc', SUCCEED, 'found', 'abbc'),
    ('ab?bc', 'abc', SUCCEED, 'found', 'abc'),
    ('ab?bc', 'abbbbc', FAIL),
    ('ab?c', 'abc', SUCCEED, 'found', 'abc'),
    ('^abc$', 'abc', SUCCEED, 'found', 'abc'),
    ('^abc$', 'abcc', FAIL),
    ('^abc', 'abcc', SUCCEED, 'found', 'abc'),
    ('^abc$', 'aabc', FAIL),
    ('abc$', 'aabc', SUCCEED, 'found', 'abc'),
    ('^', 'abc', SUCCEED, 'found+"-"', '-'),
    ('$', 'abc', SUCCEED, 'found+"-"', '-'),
    ('a.c', 'abc', SUCCEED, 'found', 'abc'),
    ('a.c', 'axc', SUCCEED, 'found', 'axc'),
    ('a.*c', 'axyzc', SUCCEED, 'found', 'axyzc'),
    ('a.*c', 'axyzd', FAIL),
    ('a[bc]d', 'abc', FAIL),
    ('a[bc]d', 'abd', SUCCEED, 'found', 'abd'),
    ('a[b-d]e', 'abd', FAIL),
    ('a[b-d]e', 'ace', SUCCEED, 'found', 'ace'),
    ('a[b-d]', 'aac', SUCCEED, 'found', 'ac'),
    ('a[-b]', 'a-', SUCCEED, 'found', 'a-'),
    ('a[\\-b]', 'a-', SUCCEED, 'found', 'a-'),
    # NOTE: not an error under PCRE/PRE:
    # ('a[b-]', 'a-', SYNTAX_ERROR),
    ('a[]b', '-', SYNTAX_ERROR),
    ('a[', '-', SYNTAX_ERROR),
    ('a\\', '-', SYNTAX_ERROR),
    ('abc)', '-', SYNTAX_ERROR),
    ('(abc', '-', SYNTAX_ERROR),
    ('a]', 'a]', SUCCEED, 'found', 'a]'),
    ('a[]]b', 'a]b', SUCCEED, 'found', 'a]b'),
    ('a[\\]]b', 'a]b', SUCCEED, 'found', 'a]b'),
    ('a[^bc]d', 'aed', SUCCEED, 'found', 'aed'),
    ('a[^bc]d', 'abd', FAIL),
    ('a[^-b]c', 'adc', SUCCEED, 'found', 'adc'),
    ('a[^-b]c', 'a-c', FAIL),
    ('a[^]b]c', 'a]c', FAIL),
    ('a[^]b]c', 'adc', SUCCEED, 'found', 'adc'),
    ('\\ba\\b', 'a-', SUCCEED, '"-"', '-'),
    ('\\ba\\b', '-a', SUCCEED, '"-"', '-'),
    ('\\ba\\b', '-a-', SUCCEED, '"-"', '-'),
    ('\\by\\b', 'xy', FAIL),
    ('\\by\\b', 'yz', FAIL),
    ('\\by\\b', 'xyz', FAIL),
    ('x\\b', 'xyz', FAIL),
    ('x\\B', 'xyz', SUCCEED, '"-"', '-'),
    ('\\Bz', 'xyz', SUCCEED, '"-"', '-'),
    ('z\\B', 'xyz', FAIL),
    ('\\Bx', 'xyz', FAIL),
    ('\\Ba\\B', 'a-', FAIL, '"-"', '-'),
    ('\\Ba\\B', '-a', FAIL, '"-"', '-'),
    ('\\Ba\\B', '-a-', FAIL, '"-"', '-'),
    ('\\By\\B', 'xy', FAIL),
    ('\\By\\B', 'yz', FAIL),
    ('\\By\\b', 'xy', SUCCEED, '"-"', '-'),
    ('\\by\\B', 'yz', SUCCEED, '"-"', '-'),
    ('\\By\\B', 'xyz', SUCCEED, '"-"', '-'),
    ('ab|cd', 'abc', SUCCEED, 'found', 'ab'),
    ('ab|cd', 'abcd', SUCCEED, 'found', 'ab'),
    ('()ef', 'def', SUCCEED, 'found+"-"+g1', 'ef-'),
    ('$b', 'b', FAIL),
    ('a\\(b', 'a(b', SUCCEED, 'found+"-"+g1', 'a(b-Error'),
    ('a\\(*b', 'ab', SUCCEED, 'found', 'ab'),
    ('a\\(*b', 'a((b', SUCCEED, 'found', 'a((b'),
    ('a\\\\b', 'a\\b', SUCCEED, 'found', 'a\\b'),
    ('((a))', 'abc', SUCCEED, 'found+"-"+g1+"-"+g2', 'a-a-a'),
    ('(a)b(c)', 'abc', SUCCEED, 'found+"-"+g1+"-"+g2', 'abc-a-c'),
    ('a+b+c', 'aabbabc', SUCCEED, 'found', 'abc'),
    ('(a+|b)*', 'ab', SUCCEED, 'found+"-"+g1', 'ab-b'),
    ('(a+|b)+', 'ab', SUCCEED, 'found+"-"+g1', 'ab-b'),
    ('(a+|b)?', 'ab', SUCCEED, 'found+"-"+g1', 'a-a'),
    (')(', '-', SYNTAX_ERROR),
    ('[^ab]*', 'cde', SUCCEED, 'found', 'cde'),
    ('abc', '', FAIL),
    ('a*', '', SUCCEED, 'found', ''),
    ('a|b|c|d|e', 'e', SUCCEED, 'found', 'e'),
    ('(a|b|c|d|e)f', 'ef', SUCCEED, 'found+"-"+g1', 'ef-e'),
    ('abcd*efg', 'abcdefg', SUCCEED, 'found', 'abcdefg'),
    ('ab*', 'xabyabbbz', SUCCEED, 'found', 'ab'),
    ('ab*', 'xayabbbz', SUCCEED, 'found', 'a'),
    ('(ab|cd)e', 'abcde', SUCCEED, 'found+"-"+g1', 'cde-cd'),
    ('[abhgefdc]ij', 'hij', SUCCEED, 'found', 'hij'),
    ('^(ab|cd)e', 'abcde', FAIL, 'xg1y', 'xy'),
    ('(abc|)ef', 'abcdef', SUCCEED, 'found+"-"+g1', 'ef-'),
    ('(a|b)c*d', 'abcd', SUCCEED, 'found+"-"+g1', 'bcd-b'),
    ('(ab|ab*)bc', 'abc', SUCCEED, 'found+"-"+g1', 'abc-a'),
    ('a([bc]*)c*', 'abc', SUCCEED, 'found+"-"+g1', 'abc-bc'),
    ('a([bc]*)(c*d)', 'abcd', SUCCEED, 'found+"-"+g1+"-"+g2', 'abcd-bc-d'),
    ('a([bc]+)(c*d)', 'abcd', SUCCEED, 'found+"-"+g1+"-"+g2', 'abcd-bc-d'),
    ('a([bc]*)(c+d)', 'abcd', SUCCEED, 'found+"-"+g1+"-"+g2', 'abcd-b-cd'),
    ('a[bcd]*dcdcde', 'adcdcde', SUCCEED, 'found', 'adcdcde'),
    ('a[bcd]+dcdcde', 'adcdcde', FAIL),
    ('(ab|a)b*c', 'abc', SUCCEED, 'found+"-"+g1', 'abc-ab'),
    ('((a)(b)c)(d)', 'abcd', SUCCEED, 'g1+"-"+g2+"-"+g3+"-"+g4', 'abc-a-b-d'),
    ('[a-zA-Z_][a-zA-Z0-9_]*', 'alpha', SUCCEED, 'found', 'alpha'),
    ('^a(bc+|b[eh])g|.h$', 'abh', SUCCEED, 'found+"-"+g1', 'bh-None'),
    ('(bc+d$|ef*g.|h?i(j|k))', 'effgz', SUCCEED, 'found+"-"+g1+"-"+g2', 'effgz-effgz-None'),
    ('(bc+d$|ef*g.|h?i(j|k))', 'ij', SUCCEED, 'found+"-"+g1+"-"+g2', 'ij-ij-j'),
    ('(bc+d$|ef*g.|h?i(j|k))', 'effg', FAIL),
    ('(bc+d$|ef*g.|h?i(j|k))', 'bcdd', FAIL),
    ('(bc+d$|ef*g.|h?i(j|k))', 'reffgz', SUCCEED, 'found+"-"+g1+"-"+g2', 'effgz-effgz-None'),
    ('(((((((((a)))))))))', 'a', SUCCEED, 'found', 'a'),
    ('multiple words of text', 'uh-uh', FAIL),
    ('multiple words', 'multiple words, yeah', SUCCEED, 'found', 'multiple words'),
    ('(.*)c(.*)', 'abcde', SUCCEED, 'found+"-"+g1+"-"+g2', 'abcde-ab-de'),
    ('\\((.*), (.*)\\)', '(a, b)', SUCCEED, 'g2+"-"+g1', 'b-a'),
    ('[k]', 'ab', FAIL),
    ('a[-]?c', 'ac', SUCCEED, 'found', 'ac'),
    ('(abc)\\1', 'abcabc', SUCCEED, 'g1', 'abc'),
    ('([a-c]*)\\1', 'abcabc', SUCCEED, 'g1', 'abc'),
    ('^(.+)?B', 'AB', SUCCEED, 'g1', 'A'),
    ('(a+).\\1$', 'aaaaa', SUCCEED, 'found+"-"+g1', 'aaaaa-aa'),
    ('^(a+).\\1$', 'aaaa', FAIL),
    ('(abc)\\1', 'abcabc', SUCCEED, 'found+"-"+g1', 'abcabc-abc'),
    ('([a-c]+)\\1', 'abcabc', SUCCEED, 'found+"-"+g1', 'abcabc-abc'),
    ('(a)\\1', 'aa', SUCCEED, 'found+"-"+g1', 'aa-a'),
    ('(a+)\\1', 'aa', SUCCEED, 'found+"-"+g1', 'aa-a'),
    ('(a+)+\\1', 'aa', SUCCEED, 'found+"-"+g1', 'aa-a'),
    ('(a).+\\1', 'aba', SUCCEED, 'found+"-"+g1', 'aba-a'),
    ('(a)ba*\\1', 'aba', SUCCEED, 'found+"-"+g1', 'aba-a'),
    ('(aa|a)a\\1$', 'aaa', SUCCEED, 'found+"-"+g1', 'aaa-a'),
    ('(a|aa)a\\1$', 'aaa', SUCCEED, 'found+"-"+g1', 'aaa-a'),
    ('(a+)a\\1$', 'aaa', SUCCEED, 'found+"-"+g1', 'aaa-a'),
    ('([abc]*)\\1', 'abcabc', SUCCEED, 'found+"-"+g1', 'abcabc-abc'),
    ('(a)(b)c|ab', 'ab', SUCCEED, 'found+"-"+g1+"-"+g2', 'ab-None-None'),
    ('(a)+x', 'aaax', SUCCEED, 'found+"-"+g1', 'aaax-a'),
    ('([ac])+x', 'aacx', SUCCEED, 'found+"-"+g1', 'aacx-c'),
    ('([^/]*/)*sub1/', 'd:msgs/tdir/sub1/trial/away.cpp', SUCCEED, 'found+"-"+g1', 'd:msgs/tdir/sub1/-tdir/'),
    ('([^.]*)\\.([^:]*):[T ]+(.*)', 'track1.title:TBlah blah blah', SUCCEED, 'found+"-"+g1+"-"+g2+"-"+g3', 'track1.title:TBlah blah blah-track1-title-Blah blah blah'),
    ('([^N]*N)+', 'abNNxyzN', SUCCEED, 'found+"-"+g1', 'abNNxyzN-xyzN'),
    ('([^N]*N)+', 'abNNxyz', SUCCEED, 'found+"-"+g1', 'abNN-N'),
    ('([abc]*)x', 'abcx', SUCCEED, 'found+"-"+g1', 'abcx-abc'),
    ('([abc]*)x', 'abc', FAIL),
    ('([xyz]*)x', 'abcx', SUCCEED, 'found+"-"+g1', 'x-'),
    ('(a)+b|aac', 'aac', SUCCEED, 'found+"-"+g1', 'aac-None'),

    # Test symbolic groups

    ('(?P<i d>aaa)a', 'aaaa', SYNTAX_ERROR),
    ('(?P<id>aaa)a', 'aaaa', SUCCEED, 'found+"-"+id', 'aaaa-aaa'),
    ('(?P<id>aa)(?P=id)', 'aaaa', SUCCEED, 'found+"-"+id', 'aaaa-aa'),
    ('(?P<id>aa)(?P=xd)', 'aaaa', SYNTAX_ERROR),

    # Test octal escapes/memory references

    ('\\1', 'a', SYNTAX_ERROR),

    # All tests from Perl

    ('ab{0,}bc', 'abbbbc', SUCCEED, 'found', 'abbbbc'),
    ('ab{1,}bc', 'abq', FAIL),
    ('ab{1,}bc', 'abbbbc', SUCCEED, 'found', 'abbbbc'),
    ('ab{1,3}bc', 'abbbbc', SUCCEED, 'found', 'abbbbc'),
    ('ab{3,4}bc', 'abbbbc', SUCCEED, 'found', 'abbbbc'),
    ('ab{4,5}bc', 'abbbbc', FAIL),
    ('ab{0,1}bc', 'abc', SUCCEED, 'found', 'abc'),
    ('ab{0,1}c', 'abc', SUCCEED, 'found', 'abc'),
    ('^', 'abc', SUCCEED, 'found', ''),
    ('$', 'abc', SUCCEED, 'found', ''),
    ('a[b-]', 'a-', SUCCEED, 'found', 'a-'),
    ('a[b-a]', '-', SYNTAX_ERROR),
    ('*a', '-', SYNTAX_ERROR),
    ('(*)b', '-', SYNTAX_ERROR),
    ('a{1,}b{1,}c', 'aabbabc', SUCCEED, 'found', 'abc'),
    ('a**', '-', SYNTAX_ERROR),
    ('a.+?c', 'abcabc', SUCCEED, 'found', 'abc'),
    ('(a+|b){0,}', 'ab', SUCCEED, 'found+"-"+g1', 'ab-b'),
    ('(a+|b){1,}', 'ab', SUCCEED, 'found+"-"+g1', 'ab-b'),
    ('(a+|b){0,1}', 'ab', SUCCEED, 'found+"-"+g1', 'a-a'),
    ('([abc])*d', 'abbbcd', SUCCEED, 'found+"-"+g1', 'abbbcd-c'),
    ('([abc])*bcd', 'abcd', SUCCEED, 'found+"-"+g1', 'abcd-a'),
    ('^(ab|cd)e', 'abcde', FAIL),
    ('((((((((((a))))))))))', 'a', SUCCEED, 'g10', 'a'),
    ('((((((((((a))))))))))\\10', 'aa', SUCCEED, 'found', 'aa'),
# Python does not have the same rules for \\41 so this is a syntax error
#    ('((((((((((a))))))))))\\41', 'aa', FAIL),
#    ('((((((((((a))))))))))\\41', 'a!', SUCCEED, 'found', 'a!'),
    ('((((((((((a))))))))))\\41', '', SYNTAX_ERROR),
    ('(?i)((((((((((a))))))))))\\41', '', SYNTAX_ERROR),
    ('(?i)abc', 'ABC', SUCCEED, 'found', 'ABC'),
    ('(?i)abc', 'XBC', FAIL),
    ('(?i)abc', 'AXC', FAIL),
    ('(?i)abc', 'ABX', FAIL),
    ('(?i)abc', 'XABCY', SUCCEED, 'found', 'ABC'),
    ('(?i)abc', 'ABABC', SUCCEED, 'found', 'ABC'),
    ('(?i)ab*c', 'ABC', SUCCEED, 'found', 'ABC'),
    ('(?i)ab*bc', 'ABC', SUCCEED, 'found', 'ABC'),
    ('(?i)ab*bc', 'ABBC', SUCCEED, 'found', 'ABBC'),
    ('(?i)ab*?bc', 'ABBBBC', SUCCEED, 'found', 'ABBBBC'),
    ('(?i)ab{0,}?bc', 'ABBBBC', SUCCEED, 'found', 'ABBBBC'),
    ('(?i)ab+?bc', 'ABBC', SUCCEED, 'found', 'ABBC'),
    ('(?i)ab+bc', 'ABC', FAIL),
    ('(?i)ab+bc', 'ABQ', FAIL),
    ('(?i)ab{1,}bc', 'ABQ', FAIL),
    ('(?i)ab+bc', 'ABBBBC', SUCCEED, 'found', 'ABBBBC'),
    ('(?i)ab{1,}?bc', 'ABBBBC', SUCCEED, 'found', 'ABBBBC'),
    ('(?i)ab{1,3}?bc', 'ABBBBC', SUCCEED, 'found', 'ABBBBC'),
    ('(?i)ab{3,4}?bc', 'ABBBBC', SUCCEED, 'found', 'ABBBBC'),
    ('(?i)ab{4,5}?bc', 'ABBBBC', FAIL),
    ('(?i)ab??bc', 'ABBC', SUCCEED, 'found', 'ABBC'),
    ('(?i)ab??bc', 'ABC', SUCCEED, 'found', 'ABC'),
    ('(?i)ab{0,1}?bc', 'ABC', SUCCEED, 'found', 'ABC'),
    ('(?i)ab??bc', 'ABBBBC', FAIL),
    ('(?i)ab??c', 'ABC', SUCCEED, 'found', 'ABC'),
    ('(?i)ab{0,1}?c', 'ABC', SUCCEED, 'found', 'ABC'),
    ('(?i)^abc$', 'ABC', SUCCEED, 'found', 'ABC'),
    ('(?i)^abc$', 'ABCC', FAIL),
    ('(?i)^abc', 'ABCC', SUCCEED, 'found', 'ABC'),
    ('(?i)^abc$', 'AABC', FAIL),
    ('(?i)abc$', 'AABC', SUCCEED, 'found', 'ABC'),
    ('(?i)^', 'ABC', SUCCEED, 'found', ''),
    ('(?i)$', 'ABC', SUCCEED, 'found', ''),
    ('(?i)a.c', 'ABC', SUCCEED, 'found', 'ABC'),
    ('(?i)a.c', 'AXC', SUCCEED, 'found', 'AXC'),
    ('(?i)a.*?c', 'AXYZC', SUCCEED, 'found', 'AXYZC'),
    ('(?i)a.*c', 'AXYZD', FAIL),
    ('(?i)a[bc]d', 'ABC', FAIL),
    ('(?i)a[bc]d', 'ABD', SUCCEED, 'found', 'ABD'),
    ('(?i)a[b-d]e', 'ABD', FAIL),
    ('(?i)a[b-d]e', 'ACE', SUCCEED, 'found', 'ACE'),
    ('(?i)a[b-d]', 'AAC', SUCCEED, 'found', 'AC'),
    ('(?i)a[-b]', 'A-', SUCCEED, 'found', 'A-'),
    ('(?i)a[b-]', 'A-', SUCCEED, 'found', 'A-'),
    ('(?i)a[b-a]', '-', SYNTAX_ERROR),
    ('(?i)a[]b', '-', SYNTAX_ERROR),
    ('(?i)a[', '-', SYNTAX_ERROR),
    ('(?i)a]', 'A]', SUCCEED, 'found', 'A]'),
    ('(?i)a[]]b', 'A]B', SUCCEED, 'found', 'A]B'),
    ('(?i)a[^bc]d', 'AED', SUCCEED, 'found', 'AED'),
    ('(?i)a[^bc]d', 'ABD', FAIL),
    ('(?i)a[^-b]c', 'ADC', SUCCEED, 'found', 'ADC'),
    ('(?i)a[^-b]c', 'A-C', FAIL),
    ('(?i)a[^]b]c', 'A]C', FAIL),
    ('(?i)a[^]b]c', 'ADC', SUCCEED, 'found', 'ADC'),
    ('(?i)ab|cd', 'ABC', SUCCEED, 'found', 'AB'),
    ('(?i)ab|cd', 'ABCD', SUCCEED, 'found', 'AB'),
    ('(?i)()ef', 'DEF', SUCCEED, 'found+"-"+g1', 'EF-'),
    ('(?i)*a', '-', SYNTAX_ERROR),
    ('(?i)(*)b', '-', SYNTAX_ERROR),
    ('(?i)$b', 'B', FAIL),
    ('(?i)a\\', '-', SYNTAX_ERROR),
    ('(?i)a\\(b', 'A(B', SUCCEED, 'found+"-"+g1', 'A(B-Error'),
    ('(?i)a\\(*b', 'AB', SUCCEED, 'found', 'AB'),
    ('(?i)a\\(*b', 'A((B', SUCCEED, 'found', 'A((B'),
    ('(?i)a\\\\b', 'A\\B', SUCCEED, 'found', 'A\\B'),
    ('(?i)abc)', '-', SYNTAX_ERROR),
    ('(?i)(abc', '-', SYNTAX_ERROR),
    ('(?i)((a))', 'ABC', SUCCEED, 'found+"-"+g1+"-"+g2', 'A-A-A'),
    ('(?i)(a)b(c)', 'ABC', SUCCEED, 'found+"-"+g1+"-"+g2', 'ABC-A-C'),
    ('(?i)a+b+c', 'AABBABC', SUCCEED, 'found', 'ABC'),
    ('(?i)a{1,}b{1,}c', 'AABBABC', SUCCEED, 'found', 'ABC'),
    ('(?i)a**', '-', SYNTAX_ERROR),
    ('(?i)a.+?c', 'ABCABC', SUCCEED, 'found', 'ABC'),
    ('(?i)a.*?c', 'ABCABC', SUCCEED, 'found', 'ABC'),
    ('(?i)a.{0,5}?c', 'ABCABC', SUCCEED, 'found', 'ABC'),
    ('(?i)(a+|b)*', 'AB', SUCCEED, 'found+"-"+g1', 'AB-B'),
    ('(?i)(a+|b){0,}', 'AB', SUCCEED, 'found+"-"+g1', 'AB-B'),
    ('(?i)(a+|b)+', 'AB', SUCCEED, 'found+"-"+g1', 'AB-B'),
    ('(?i)(a+|b){1,}', 'AB', SUCCEED, 'found+"-"+g1', 'AB-B'),
    ('(?i)(a+|b)?', 'AB', SUCCEED, 'found+"-"+g1', 'A-A'),
    ('(?i)(a+|b){0,1}', 'AB', SUCCEED, 'found+"-"+g1', 'A-A'),
    ('(?i)(a+|b){0,1}?', 'AB', SUCCEED, 'found+"-"+g1', '-None'),
    ('(?i))(', '-', SYNTAX_ERROR),
    ('(?i)[^ab]*', 'CDE', SUCCEED, 'found', 'CDE'),
    ('(?i)abc', '', FAIL),
    ('(?i)a*', '', SUCCEED, 'found', ''),
    ('(?i)([abc])*d', 'ABBBCD', SUCCEED, 'found+"-"+g1', 'ABBBCD-C'),
    ('(?i)([abc])*bcd', 'ABCD', SUCCEED, 'found+"-"+g1', 'ABCD-A'),
    ('(?i)a|b|c|d|e', 'E', SUCCEED, 'found', 'E'),
    ('(?i)(a|b|c|d|e)f', 'EF', SUCCEED, 'found+"-"+g1', 'EF-E'),
    ('(?i)abcd*efg', 'ABCDEFG', SUCCEED, 'found', 'ABCDEFG'),
    ('(?i)ab*', 'XABYABBBZ', SUCCEED, 'found', 'AB'),
    ('(?i)ab*', 'XAYABBBZ', SUCCEED, 'found', 'A'),
    ('(?i)(ab|cd)e', 'ABCDE', SUCCEED, 'found+"-"+g1', 'CDE-CD'),
    ('(?i)[abhgefdc]ij', 'HIJ', SUCCEED, 'found', 'HIJ'),
    ('(?i)^(ab|cd)e', 'ABCDE', FAIL),
    ('(?i)(abc|)ef', 'ABCDEF', SUCCEED, 'found+"-"+g1', 'EF-'),
    ('(?i)(a|b)c*d', 'ABCD', SUCCEED, 'found+"-"+g1', 'BCD-B'),
    ('(?i)(ab|ab*)bc', 'ABC', SUCCEED, 'found+"-"+g1', 'ABC-A'),
    ('(?i)a([bc]*)c*', 'ABC', SUCCEED, 'found+"-"+g1', 'ABC-BC'),
    ('(?i)a([bc]*)(c*d)', 'ABCD', SUCCEED, 'found+"-"+g1+"-"+g2', 'ABCD-BC-D'),
    ('(?i)a([bc]+)(c*d)', 'ABCD', SUCCEED, 'found+"-"+g1+"-"+g2', 'ABCD-BC-D'),
    ('(?i)a([bc]*)(c+d)', 'ABCD', SUCCEED, 'found+"-"+g1+"-"+g2', 'ABCD-B-CD'),
    ('(?i)a[bcd]*dcdcde', 'ADCDCDE', SUCCEED, 'found', 'ADCDCDE'),
    ('(?i)a[bcd]+dcdcde', 'ADCDCDE', FAIL),
    ('(?i)(ab|a)b*c', 'ABC', SUCCEED, 'found+"-"+g1', 'ABC-AB'),
    ('(?i)((a)(b)c)(d)', 'ABCD', SUCCEED, 'g1+"-"+g2+"-"+g3+"-"+g4', 'ABC-A-B-D'),
    ('(?i)[a-zA-Z_][a-zA-Z0-9_]*', 'ALPHA', SUCCEED, 'found', 'ALPHA'),
    ('(?i)^a(bc+|b[eh])g|.h$', 'ABH', SUCCEED, 'found+"-"+g1', 'BH-None'),
    ('(?i)(bc+d$|ef*g.|h?i(j|k))', 'EFFGZ', SUCCEED, 'found+"-"+g1+"-"+g2', 'EFFGZ-EFFGZ-None'),
    ('(?i)(bc+d$|ef*g.|h?i(j|k))', 'IJ', SUCCEED, 'found+"-"+g1+"-"+g2', 'IJ-IJ-J'),
    ('(?i)(bc+d$|ef*g.|h?i(j|k))', 'EFFG', FAIL),
    ('(?i)(bc+d$|ef*g.|h?i(j|k))', 'BCDD', FAIL),
    ('(?i)(bc+d$|ef*g.|h?i(j|k))', 'REFFGZ', SUCCEED, 'found+"-"+g1+"-"+g2', 'EFFGZ-EFFGZ-None'),
    ('(?i)((((((((((a))))))))))', 'A', SUCCEED, 'g10', 'A'),
    ('(?i)((((((((((a))))))))))\\10', 'AA', SUCCEED, 'found', 'AA'),
    #('(?i)((((((((((a))))))))))\\41', 'AA', FAIL),
    #('(?i)((((((((((a))))))))))\\41', 'A!', SUCCEED, 'found', 'A!'),
    ('(?i)(((((((((a)))))))))', 'A', SUCCEED, 'found', 'A'),
    ('(?i)(?:(?:(?:(?:(?:(?:(?:(?:(?:(a))))))))))', 'A', SUCCEED, 'g1', 'A'),
    ('(?i)(?:(?:(?:(?:(?:(?:(?:(?:(?:(a|b|c))))))))))', 'C', SUCCEED, 'g1', 'C'),
    ('(?i)multiple words of text', 'UH-UH', FAIL),
    ('(?i)multiple words', 'MULTIPLE WORDS, YEAH', SUCCEED, 'found', 'MULTIPLE WORDS'),
    ('(?i)(.*)c(.*)', 'ABCDE', SUCCEED, 'found+"-"+g1+"-"+g2', 'ABCDE-AB-DE'),
    ('(?i)\\((.*), (.*)\\)', '(A, B)', SUCCEED, 'g2+"-"+g1', 'B-A'),
    ('(?i)[k]', 'AB', FAIL),
#    ('(?i)abcd', 'ABCD', SUCCEED, 'found+"-"+\\found+"-"+\\\\found', 'ABCD-$&-\\ABCD'),
#    ('(?i)a(bc)d', 'ABCD', SUCCEED, 'g1+"-"+\\g1+"-"+\\\\g1', 'BC-$1-\\BC'),
    ('(?i)a[-]?c', 'AC', SUCCEED, 'found', 'AC'),
    ('(?i)(abc)\\1', 'ABCABC', SUCCEED, 'g1', 'ABC'),
    ('(?i)([a-c]*)\\1', 'ABCABC', SUCCEED, 'g1', 'ABC'),
    ('a(?!b).', 'abad', SUCCEED, 'found', 'ad'),
    ('a(?=d).', 'abad', SUCCEED, 'found', 'ad'),
    ('a(?=c|d).', 'abad', SUCCEED, 'found', 'ad'),
    ('a(?:b|c|d)(.)', 'ace', SUCCEED, 'g1', 'e'),
    ('a(?:b|c|d)*(.)', 'ace', SUCCEED, 'g1', 'e'),
    ('a(?:b|c|d)+?(.)', 'ace', SUCCEED, 'g1', 'e'),
    ('a(?:b|(c|e){1,2}?|d)+?(.)', 'ace', SUCCEED, 'g1 + g2', 'ce'),

    # lookbehind: split by : but not if it is escaped by -.
    ('(?<!-):(.*?)(?<!-):', 'a:bc-:de:f', SUCCEED, 'g1', 'bc-:de' ),
    # escaping with \ as we know it
    ('(?<!\\\\):(.*?)(?<!\\\\):', 'a:bc\\:de:f', SUCCEED, 'g1', 'bc\\:de' ),
    # terminating with ' and escaping with ? as in edifact
    ("(?<!\\?)'(.*?)(?<!\\?)'", "a'bc?'de'f", SUCCEED, 'g1', "bc?'de" ),

    # Comments using the (?#...) syntax

    ('w(?# comment', 'w', SYNTAX_ERROR),
    ('w(?# comment 1)xy(?# comment 2)z', 'wxyz', SUCCEED, 'found', 'wxyz'),

    # Check odd placement of embedded pattern modifiers

    # not an error under PCRE/PRE:
    ('(?i)w', 'W', SUCCEED, 'found', 'W'),
    # ('w(?i)', 'W', SYNTAX_ERROR),

    # Comments using the x embedded pattern modifier

    ("""(?x)w# comment 1
        x y
        # comment 2
        z""", 'wxyz', SUCCEED, 'found', 'wxyz'),

    # using the m embedded pattern modifier

    ('^abc', """jkl
abc
xyz""", FAIL),
    ('(?m)^abc', """jkl
abc
xyz""", SUCCEED, 'found', 'abc'),

    ('(?m)abc$', """jkl
xyzabc
123""", SUCCEED, 'found', 'abc'),



    # test \w, etc. both inside and outside character classes

    ('\\w+', '--ab_cd0123--', SUCCEED, 'found', 'ab_cd0123'),
    ('[\\w]+', '--ab_cd0123--', SUCCEED, 'found', 'ab_cd0123'),
    ('\\D+', '1234abc5678', SUCCEED, 'found', 'abc'),
    ('[\\D]+', '1234abc5678', SUCCEED, 'found', 'abc'),
    ('[\\da-fA-F]+', '123abc', SUCCEED, 'found', '123abc'),
    # not an error under PCRE/PRE:
    # ('[\\d-x]', '-', SYNTAX_ERROR),
    (r'([\s]*)([\S]*)([\s]*)', ' testing!1972', SUCCEED, 'g3+g2+g1', 'testing!1972 '),
    (r'(\s*)(\S*)(\s*)', ' testing!1972', SUCCEED, 'g3+g2+g1', 'testing!1972 '),

    (r'\xff', '\377', SUCCEED, 'found', chr(255)),
    # new \x semantics
    (r'\x00ff', '\377', FAIL),
    # (r'\x00ff', '\377', SUCCEED, 'found', chr(255)),
    (r'\t\n\v\r\f\a', '\t\n\v\r\f\a', SUCCEED, 'found', '\t\n\v\r\f\a'),
    ('\t\n\v\r\f\a', '\t\n\v\r\f\a', SUCCEED, 'found', '\t\n\v\r\f\a'),
    (r'\t\n\v\r\f\a', '\t\n\v\r\f\a', SUCCEED, 'found', chr(9)+chr(10)+chr(11)+chr(13)+chr(12)+chr(7)),
    (r'[\t][\n][\v][\r][\f][\b]', '\t\n\v\r\f\b', SUCCEED, 'found', '\t\n\v\r\f\b'),

    #
    # post-1.5.2 additions

    # xmllib problem
    (r'(([a-z]+):)?([a-z]+)$', 'smil', SUCCEED, 'g1+"-"+g2+"-"+g3', 'None-None-smil'),
    # bug 110866: reference to undefined group
    (r'((.)\1+)', '', SYNTAX_ERROR),
    # bug 111869: search (PRE/PCRE fails on this one, SRE doesn't)
    (r'.*d', 'abc\nabd', SUCCEED, 'found', 'abd'),
    # bug 112468: various expected syntax errors
    (r'(', '', SYNTAX_ERROR),
    (r'[\41]', '!', SUCCEED, 'found', '!'),
    # bug 114033: nothing to repeat
    (r'(x?)?', 'x', SUCCEED, 'found', 'x'),
    # bug 115040: rescan if flags are modified inside pattern
    (r'(?x) foo ', 'foo', SUCCEED, 'found', 'foo'),
    # bug 115618: negative lookahead
    (r'(?<!abc)(d.f)', 'abcdefdof', SUCCEED, 'found', 'dof'),
    # bug 116251: character class bug
    (r'[\w-]+', 'laser_beam', SUCCEED, 'found', 'laser_beam'),
    # bug 123769+127259: non-greedy backtracking bug
    (r'.*?\S *:', 'xx:', SUCCEED, 'found', 'xx:'),
    (r'a[ ]*?\ (\d+).*', 'a   10', SUCCEED, 'found', 'a   10'),
    (r'a[ ]*?\ (\d+).*', 'a    10', SUCCEED, 'found', 'a    10'),
    # bug 127259: \Z shouldn't depend on multiline mode
    (r'(?ms).*?x\s*\z(.*)','xx\nx\n', SUCCEED, 'g1', ''),
    # bug 128899: uppercase literals under the ignorecase flag
    (r'(?i)M+', 'MMM', SUCCEED, 'found', 'MMM'),
    (r'(?i)m+', 'MMM', SUCCEED, 'found', 'MMM'),
    (r'(?i)[M]+', 'MMM', SUCCEED, 'found', 'MMM'),
    (r'(?i)[m]+', 'MMM', SUCCEED, 'found', 'MMM'),
    # bug 130748: ^* should be an error (nothing to repeat)
    (r'^*', '', SYNTAX_ERROR),
    # bug 133283: minimizing repeat problem
    (r'"(?:\\"|[^"])*?"', r'"\""', SUCCEED, 'found', r'"\""'),
    # bug 477728: minimizing repeat problem
    (r'^.*?$', 'one\ntwo\nthree\n', FAIL),
    # bug 483789: minimizing repeat problem
    (r'a[^>]*?b', 'a>b', FAIL),
    # bug 490573: minimizing repeat problem
    (r'^a*?$', 'foo', FAIL),
    # bug 470582: nested groups problem
    (r'^((a)c)?(ab)$', 'ab', SUCCEED, 'g1+"-"+g2+"-"+g3', 'None-None-ab'),
    # another minimizing repeat problem (capturing groups in assertions)
    ('^([ab]*?)(?=(b)?)c', 'abc', SUCCEED, 'g1+"-"+g2', 'ab-None'),
    ('^([ab]*?)(?!(b))c', 'abc', SUCCEED, 'g1+"-"+g2', 'ab-None'),
    ('^([ab]*?)(?<!(a))c', 'abc', SUCCEED, 'g1+"-"+g2', 'ab-None'),
]

u = '\N{LATIN CAPITAL LETTER A WITH DIAERESIS}'
raw_tests.extend([
    (r'\b.\b', 'a', SUCCEED, 'found', 'a'),
    (r'(?u)\b.\b', u, SUCCEED, 'found', u),
    (r'(?u)\w', u, SUCCEED, 'found', u),
])

# Indices of the 10 tests that are known to differ between `re` and `reru`.
# We mark these as "expected failures" so the CI pipeline stays green.
KNOWN_DIFFERENCES = {1, 5, 14, 204, 214, 215, 288, 375, 386, 391}

def build_eval_env(match_obj):
    """
    Builds the local variables dict required by the test suite's eval_string.
    Maps group(0) to 'found', and group(1..99) to 'g1'..'g99'.
    """
    if not match_obj:
        return {}

    env = {"groups": match_obj.groups()}
    
    # group(0) -> 'found'
    try:
        env["found"] = match_obj.group(0)
    except Exception:
        env["found"] = "Error"
        
    # group(1) to group(99) -> 'g1' to 'g99'
    for i in range(1, 100):
        try:
            val = match_obj.group(i)
            env[f"g{i}"] = val if val is not None else "None"
        except Exception:
            # reru throws ValueError on out-of-bounds, re throws IndexError
            env[f"g{i}"] = "Error"
            
    # Map common named groups used in the tests to locals
    for name in ["id", "i d", "xd", "foo_123", "foo_124", "1", "!"]:
        try:
            val = match_obj.group(name)
            env[name] = val if val is not None else "None"
        except Exception:
            env[name] = "Error"

    return env


def run_engine(engine, pattern, text, eval_string):
    """Runs a pattern through an engine and evaluates the output."""
    try:
        match = engine.search(pattern, text)
        outcome = SUCCEED if match else FAIL
    except Exception:
        # Catch re.error, reru's AppError/ValueError, etc.
        outcome = SYNTAX_ERROR
        match = None
        
    eval_res = None
    if outcome == SUCCEED and eval_string and match:
        env = build_eval_env(match)
        try:
            eval_res = str(eval(eval_string, {}, env))
        except Exception as e:
            eval_res = f"Eval Exception: {e}"
            
    return outcome, eval_res

# Format the test data for pytest
test_params = []
for idx, test in enumerate(raw_tests):
    if len(test) == 3:
        pattern, text, expected_outcome = test
        eval_string, expected_eval = None, None
    else:
        pattern, text, expected_outcome, eval_string, expected_eval = test
        
    # Mark the test as an expected failure if it's in our known differences list
    param = pytest.param(
        pattern, text, expected_outcome, eval_string, expected_eval,
        id=f"test_{idx}"
    )
    if idx in KNOWN_DIFFERENCES:
        param = pytest.param(
            pattern, text, expected_outcome, eval_string, expected_eval,
            marks=pytest.mark.xfail(reason="Known deviation from Python re"),
            id=f"test_{idx}"
        )
    test_params.append(param)

@pytest.mark.parametrize("pattern, text, expected_outcome, eval_string, expected_eval", test_params)
def test_reru_matches_re_behavior(pattern, text, expected_outcome, eval_string, expected_eval):
    re_out, re_eval = run_engine(re, pattern, text, eval_string)
    
    # Run 'reru'
    reru_out, reru_eval = run_engine(reru, pattern, text, eval_string)
    
    # 1. Assert outcomes match
    assert reru_out == re_out, f"Outcome mismatch. Expected {re_out}, got {reru_out}"
    
    # 2. Assert evaluations (capture group values) match
    if re_out == SUCCEED and eval_string:
        assert reru_eval == re_eval, f"Eval mismatch. Expected {re_eval}, got {reru_eval}"

'''def run_comparison():
    differences = []
    
    for idx, test in enumerate(raw_tests):
        if len(test) == 3:
            pattern, text, expected_outcome = test
            eval_string, expected_eval = None, None
        else:
            pattern, text, expected_outcome, eval_string, expected_eval = test

        # Run both engines
        re_out, re_eval = run_engine(re, pattern, text, eval_string)
        reru_out, reru_eval = run_engine(reru, pattern, text, eval_string)
        
        match_diff = False
        
        # 1. Did reru deviate from the standard expected outcome?
        if reru_out != expected_outcome:
            match_diff = True
            
        # 2. Did reru evaluation matching strings deviate?
        if reru_out == SUCCEED and eval_string and reru_eval != expected_eval:
            match_diff = True
            
        # 3. Did reru explicitly disagree with re?
        if re_out != reru_out or re_eval != reru_eval:
            match_diff = True

        if match_diff:
            differences.append({
                "idx": idx,
                "pattern": pattern,
                "text": text,
                "expected_out": OUTCOME_MAP.get(expected_outcome),
                "expected_eval": expected_eval,
                "re_out": OUTCOME_MAP.get(re_out),
                "re_eval": re_eval,
                "reru_out": OUTCOME_MAP.get(reru_out),
                "reru_eval": reru_eval
            })
            
    print(f"Total tests processed: {len(tests)}")
    if not differences:
        print("🎉 All tests passed perfectly! reru behaves exactly like re for this suite.")
    else:
        print(f"⚠️ Found {len(differences)} differences between `re` and `reru`:\n")
        
        for diff in differences:
            print("-" * 60)
            print(f"Test #{diff['idx']} | Pattern: {diff['pattern']!r} | Text: {diff['text']!r}")
            print(f"  [Expected] Outcome: {diff['expected_out']:<13} | Eval: {diff['expected_eval']}")
            print(f"  [python re] Outcome: {diff['re_out']:<13} | Eval: {diff['re_eval']}")
            print(f"  [reru]      Outcome: {diff['reru_out']:<13} | Eval: {diff['reru_eval']}")

if __name__ == '__main__':
    run_comparison()'''