from typing import Optional, List

class Match:
    def start(self) -> int: ...
    def end(self) -> int: ...
    def group(self, _i: int = 0) -> str: ...

class Pattern:
    def is_match(self, text: str) -> bool: ...
    def match(self, text: str) -> Optional[Match]: ...
    def search(self, text: str) -> Optional[Match]: ...


class ReConfig:
    case_insensitive: bool
    ignore_whitespace: bool
    multiline: bool
    unicode_mode: bool
    size_limit: Optional[int]
    dfa_size_limit: int
    backtrack_limit: Optional[int]

    def __init__(self, 
        case_insensitive: bool, ignore_whitespace: bool, multiline: bool, unicode_mode: bool,
        size_limit: Optional[int], dfa_size_limit: int, backtrack_limit: Optional[int]
    ) -> None: ...


class ReRu:
    def compile(pattern: str, config: Optional[ReConfig] = None) -> Pattern: ...
    def is_match(pattern: str, text: str, config: Optional[ReConfig] = None) -> bool: ...
    def match(pattern: str, text: str, config: Optional[ReConfig] = None) -> Optional[Match]: ...
    def search(pattern: str, text: str, config: Optional[ReConfig] = None) -> Optional[Match]: ...



__version__: str
__name__: str
__package__: str
__all__: List[str]