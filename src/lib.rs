use std::sync::{Arc,Mutex};
use std::sync::atomic::{AtomicI32,AtomicBool,AtomicUsize,Ordering};

use pyo3::types::PyString;
use pyo3::{class, prelude::*};
use pyo3::exceptions::PyValueError;
use {
    once_cell::sync::Lazy,
    regex::{Regex, RegexBuilder}, // no backtrack
    fancy_regex::{Regex as Regex2, RegexBuilder as RegexBuilder2}, // backtrack
    dashmap::DashMap,
};


#[pyclass(frozen)]
struct Match{
    start_pos: usize,
    end_pos: usize,
    content: String,
}
#[pymethods]
impl Match {
    // This allows: m.start()
    fn start(&self) -> usize { self.start_pos }

    // This allows: m.end()
    fn end(&self) -> usize { self.end_pos }

    // This allows: m.group() or m.group(0)
    #[pyo3(signature = (_i=0))]
    fn group(&self, _i: i32) -> &str {
        self.content.as_str()
    }
}


pub trait PyRe {
    fn search(&self, text: &str) -> PyResult<Option<Match>>;
}


#[pyclass]
struct Re1 {
 re: Regex
}

#[pymethods]
impl Re1 {
    #[inline(always)]
    pub fn is_match(&self, text: &str) -> PyResult<bool> {
        Ok(self.re.is_match(text))
    }

    #[inline(always)]
    pub fn search(&self, text: &str) -> PyResult<Option<Match>> {
        match self.re.find(text) {
            Some(m) => {
                Ok(Some(Match { start_pos: m.start(), end_pos: m.end(), content: m.as_str().to_string() }))
            },
            None => Ok(None),
        }
    }
}

enum ReType {
    Re1(Re1),
    Re2(Re2),
}

impl PyRe for ReType {

    #[inline(always)]
    fn search(&self, text: &str) -> PyResult<Option<Match>> {
        match self {
            ReType::Re1(re1) => re1.search(text),
            ReType::Re2(re2) => re2.search(text),
        }
    }
}
impl ReType {
    #[inline(always)]
    fn is_match(&self, text: &str) -> PyResult<bool> {
        match self {
            ReType::Re1(re1) => re1.is_match(text),
            ReType::Re2(re2) => re2.is_match(text),
        }
    }
}
#[pyclass]
struct Re2 {
    re: Regex2
}

#[pymethods]
impl Re2 {
    #[inline(always)]
    fn is_match(&self, text: &str) -> PyResult<bool> {
        Ok(self.re.is_match(text).map_err(|e| PyValueError::new_err(format!("Regex error: {}", e)))?)
    }
    #[inline(always)]
    fn search(&self, text: &str) -> PyResult<Option<Match>> {
        match self.re.find(text).map_err(|e| PyValueError::new_err(format!("Regex error: {}", e)))?{
            Some(m) => {
                Ok(Some(Match { start_pos: m.start(), end_pos: m.end(), content: m.as_str().to_string() }))
            },
            None => Ok(None),
        }
    }
}

static RE_CACHE: Lazy<DashMap<String, Arc<ReType>>> = Lazy::new(|| {
    DashMap::new()
});


fn is_fancy_regex(pattern: &str) -> bool {
    let bytes = pattern.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        match bytes[i] {
            // Handle Backslashes (Potential Backreferences or Escapes)
            b'\\' => {
                if i + 1 < len {
                    let next_char = bytes[i + 1];
                    // Check for backreferences \1 through \9
                    if next_char >= b'1' && next_char <= b'9' {
                        return true;
                    }
                    // Skip the next character because it is escaped.
                    // This prevents identifying \( as a start of a group.
                    i += 1; 
                }
            }
            // Handle Open Parentheses (Potential Lookarounds)
            b'(' => {
                // We need at least 3 chars for the shortest lookaround: (?=
                if i + 2 < len && bytes[i + 1] == b'?' {
                    let third = bytes[i + 2];
                    match third {
                        // Check for (?= and (?!
                        b'=' | b'!' => return true,
                        // Check for (?<= and (?<!
                        b'<' => {
                            if i + 3 < len {
                                let fourth = bytes[i + 3];
                                if fourth == b'=' || fourth == b'!' {
                                    return true;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
    false
}

#[pyclass]
struct ReConfig {
    case_insensitive: bool,
    ignore_whitespace: bool,
    multiline: bool,
    unicode_mode: bool,
    size_limit: Option<usize>,
    dfa_size_limit: usize,
    backtrack_limit: Option<usize>,
}
#[pymethods]
impl ReConfig {
    #[new]
    fn new(
        case_insensitive: bool, ignore_whitespace: bool, multiline: bool, unicode_mode: bool,
        size_limit: Option<usize>, dfa_size_limit: usize, backtrack_limit: Option<usize>
    ) -> Self {
        ReConfig {
            case_insensitive,
            ignore_whitespace,
            multiline,
            unicode_mode,
            size_limit,
            dfa_size_limit,
            backtrack_limit,
        }
    }
}

#[inline]
fn compile_regex(pattern: &str, config: Option<&ReConfig>) -> PyResult<Arc<ReType>> {
    let is_fancy = is_fancy_regex(pattern);
    let re_enum = if is_fancy {
        let mut re = RegexBuilder2::new(pattern);
        let re = match config {
            Some(cfg) => {
                let mut re = re
                    .multi_line(cfg.multiline)
                    .case_insensitive(cfg.case_insensitive)
                    .ignore_whitespace(cfg.ignore_whitespace)
                    .unicode_mode(cfg.unicode_mode)
                    .delegate_dfa_size_limit(cfg.dfa_size_limit);
                if let Some(backtrack_limit) = cfg.backtrack_limit {
                    re = re.backtrack_limit(backtrack_limit);
                }
                if let Some(size_limit) = cfg.size_limit {
                    re = re.delegate_size_limit(size_limit);
                }
                re.build().map_err(|e| PyValueError::new_err(format!("Regex error: {}", e)))?
            },
            None => re.build().map_err(|e| PyValueError::new_err(format!("Regex error: {}", e)))?
        };
        ReType::Re2(Re2 { re })
    } else {
        let mut re = RegexBuilder::new(pattern);
        let re = match config {
            Some(cfg) => {
                let mut re = re
                    .multi_line(cfg.multiline)
                    .case_insensitive(cfg.case_insensitive)
                    .ignore_whitespace(cfg.ignore_whitespace)
                    .unicode(cfg.unicode_mode)
                    .dfa_size_limit(cfg.dfa_size_limit);
                if let Some(size_limit) = cfg.size_limit {
                    re = re.size_limit(size_limit);
                }
                re.build().map_err(|e| PyValueError::new_err(format!("Regex error: {}", e)))?
            },
            None => re.build().map_err(|e| PyValueError::new_err(format!("Regex error: {}", e)))?
        };
        ReType::Re1(Re1 { re })
    };
    Ok(Arc::new(re_enum))
}

#[pyclass]
struct ReRu {
}

#[pymethods]
impl ReRu {
    #[new]
    fn py_new() -> Self {
        ReRu {}
    }

    #[staticmethod]
    #[pyo3(signature = (pattern, text, config=None))]
    pub fn is_match(pattern: &str, text: &str, config: Option<&ReConfig>) -> PyResult<bool> {
        if let Some(re_arc) = RE_CACHE.get(pattern) {
            return re_arc.is_match(text);
        }

        let re_arc = compile_regex(pattern, config)?;
        
        {
            RE_CACHE.entry(pattern.to_string())
            .or_insert(re_arc.clone());
        }
        re_arc.is_match(text)
    }
    #[staticmethod]
    #[pyo3(signature = (pattern, text, config=None))]
    pub fn search(pattern: &str, text: &str, config: Option<&ReConfig>) -> PyResult<Option<Match>> {
        if let Some(re_arc) = RE_CACHE.get(pattern) {
            return re_arc.search(text);
        }
        let re_arc = compile_regex(pattern, config)?;
        {
            let stored_ref = RE_CACHE.entry(pattern.to_string())
            .or_insert(re_arc.clone());
        }
        re_arc.search(text)
    }
}

#[pymodule]
fn reru(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Match>()?;
    m.add_class::<ReConfig>()?;
    m.add_class::<ReRu>()?;
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    m.add("__version__", VERSION)?;
    Ok(())
}
