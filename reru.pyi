from typing import Optional

class Match:
    def start(self) -> int: ...
    def end(self) -> int: ...
    def group(self, _i: int = 0) -> str: ...

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
    def is_match(pattern: str, text: str, config: Optional[ReConfig] = None) -> bool: ...
    def search(pattern: str, text: str, config: Optional[ReConfig] = None) -> Optional[Match]: ...