from typing import Optional, List, Union

class SelectEngine:
    Std: int
    Fancy: int

class Match:
    """
    Represents a successful regex match.
    contains the original text and span indices for the match and capture groups.
    """
    def start(self) -> int:
        """
        Returns the starting index of the match.
        """

    def end(self) -> int:
        """
        Returns the ending index of the match.
        """

    def group(self, ident: Union[int, str] = 0) -> str:
        """
        Returns the substring matched by the given group.

        Args:
            ident: The group index (int) or group name (str). Defaults to 0 (the whole match).

        Returns:
            The matched string.

        Raises:
            ValueError: If the group index/name is invalid or not found.
        """

    def groups(self) -> List[Optional[str]]:
        """
        Returns a list of all capture groups (excluding the specific whole-match group 0).

        Returns:
            A list where each element is the string matched by the group, or None if the group did not participate.
        """

    def lastindex(self) -> int:
        """
        Returns the integer index of the last matched capturing group.
        """

class Pattern:
    """
    A compiled regular expression object.
    
    This object holds a thread-safe reference to the underlying Rust Regex engine.
    It automatically handles switching between the standard `regex` crate (O(n) time)
    and `fancy-regex` (for look-arounds and back-references) based on the pattern.
    """

    def engine_info(self) -> str:
        """
        Returns the name of the underlying engine being used ('regex' or 'fancy_regex').
        """

    def group_names(self) -> List[str]:
        """
        Returns a list of named capture groups defined in the pattern.
        """

    def is_match(self, text: str) -> bool:
        """
        Checks if the pattern matches the string at the beginning.

        This is faster than `match()` as it returns a boolean without allocating a Match object.

        Returns:
            True if the pattern matches at the start of `text`.
        """
    def is_search(self, text: str) -> bool:
        """
        Checks if the pattern matches anywhere in the string.

        This is faster than `search()` as it returns a boolean without allocating a Match object.

        Returns:
            True if the pattern is found anywhere in `text`.
        """

    def match(self, text: str) -> Optional[Match]:
        """
        Attempts to match the pattern at the beginning of the string.

        Returns:
            A Match object if found, otherwise None.
        """
    def search(self, text: str) -> Optional[Match]:
        """
        Searches for the pattern anywhere in the string.

        Returns:
            A Match object if found, otherwise None.
        """
    def find(self, text: str) -> Optional[Match]:
        """
        Finds the first occurrence of the pattern in the string.

        Returns:
            A Match object if found, otherwise None.
        """
    def findall(self, text: str) -> List[str]:
        """
        Finds all non-overlapping occurrences of the pattern in the string.
        """

    def sub(self, repl: str, text: str) -> str:
        """
        Return the string obtained by replacing the leftmost non-overlapping occurrences
        of the pattern in string by the replacement `repl`.

        Args:
            repl: The replacement string.
            text: The input string to perform replacements on.
        Returns:
            The modified string with replacements.
        """

    @staticmethod
    def escape(text: str) -> str:
        """
        Escape special characters in a string.
        """


class ReConfig:
    """
    Configuration options for compiling a regex pattern.
    """
    case_insensitive: bool
    ignore_whitespace: bool
    multiline: bool
    unicode_mode: bool
    size_limit: Optional[int]
    dfa_size_limit: int
    backtrack_limit: Optional[int]

    def __init__(self, 
        case_insensitive: bool = False,
        ignore_whitespace: bool = False,
        multiline: bool = False,
        unicode_mode: bool = False,
        size_limit: Optional[int] = None,
        dfa_size_limit: int = 10_000_000,
        backtrack_limit: Optional[int] = None
    ) -> None:
        """
        Args:
            case_insensitive: Enable case-insensitive matching.
            ignore_whitespace: Allow whitespace and comments in pattern.
            multiline: ^ and $ match start/end of line.
            unicode_mode: Enable Unicode support.
            size_limit: Limit the size of the compiled regex.
            dfa_size_limit: Limit the size of the DFA graph (std engine only).
            backtrack_limit: Limit the backtrack stack (fancy engine only).
        """


def compile(pattern: str, config: Optional[ReConfig] = None) -> Pattern:
    """
    Compile a regular expression pattern into a Pattern object.

    This function utilizes a thread-safe cache. If the pattern (and config) 
    has been seen before, a cached Pattern is returned immediately.

    Args:
        pattern: The regex string.
        config: Optional configuration object.

    Returns:
        A compiled Pattern object.
    """

def compile_custom(
    pattern: str, 
    config: Optional[ReConfig] = None, 
    select_engine: Optional[SelectEngine] = None
) -> Pattern:
    """
    Compile a regex pattern, forcing a specific underlying engine.

    Args:
        pattern: The regex string.
        config: Optional configuration.
        select_engine: Force usage of 'Std' (Rust Regex) or 'Fancy' (FancyRegex).
                       If None, auto-detection is used.
    """

def is_match(pattern: str, text: str, config: Optional[ReConfig] = None) -> bool:
    """
    Checks if the pattern matches the string at the beginning.

    This is faster than `match()` as it returns a boolean without allocating a Match object.

    Args:
        pattern: The regex string.
        text: The input string to match against.
        config: Optional configuration.
    Returns:
        True if the pattern matches at the start of `text`.
    """
def is_search(pattern: str, text: str, config: Optional[ReConfig] = None) -> bool: 
    """
    Checks if the pattern matches anywhere in the string.

    This is faster than `search()` as it returns a boolean without allocating a Match object.

    Args:
        pattern: The regex string.
        text: The input string to match against.
        config: Optional configuration.
    Returns:
        True if the pattern is found anywhere in `text`.
    """

def match(pattern: str, text: str, config: Optional[ReConfig] = None) -> Optional[Match]:
    """
    Attempts to match the pattern at the beginning of the string.

    Args:
        pattern: The regex string.
        text: The input string to match against.
        config: Optional configuration.
    Returns:
        A Match object if found, otherwise None.
    """

def search(pattern: str, text: str, config: Optional[ReConfig] = None) -> Optional[Match]:
    """
    Searches for the pattern anywhere in the string.

    Args:
        pattern: The regex string.
        text: The input string to match against.
        config: Optional configuration.
    Returns:
        A Match object if found, otherwise None.
    """

def sub(pattern: str, repl: str, text: str, config: Optional[ReConfig] = None) -> str:
    """
    Return the string obtained by replacing the leftmost non-overlapping occurrences
    of the pattern in string by the replacement `repl`.

    Args:
        pattern: The regex string.
        repl: The replacement string.
        text: The input string to perform replacements on.
        config: Optional configuration.
    Returns:
        The modified string with replacements.
    """

def escape(text: str) -> str:
    """
    Escape special characters in a string.
    """



__version__: str
__name__: str
__package__: str
__all__: List[str]