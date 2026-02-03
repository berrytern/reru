use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use pyo3::{prelude::*};
use pyo3::exceptions::PyValueError;
use pyo3::types::PyString;
use regex::{Regex, RegexBuilder};
use fancy_regex::{Regex as Regex2, RegexBuilder as RegexBuilder2};
use smallvec::{SmallVec,smallvec};
mod exceptions;
use exceptions::AppError;

use crate::exceptions::ReError;


type SpanVec = SmallVec<[(usize, usize); 8]>;

#[pyclass(frozen, freelist = 100)]
pub struct Match {
    text: Py<PyString>, 
    spans: SpanVec,
}

pub struct RuMatch {
    text: String, 
    spans: SpanVec,
}

impl From<RuMatch> for Match {
    fn from(rm: RuMatch) -> Self {
        Python::attach(|py| {
            Match {
                text: PyString::new(py, &rm.text).into(),
                spans: rm.spans,
            }
        })
    }
}

#[pymethods]
impl Match {
    fn start(&self) -> usize {
        self.spans.first().map(|(s, _)| *s).unwrap_or(0)
    }

    fn end(&self) -> usize {
        self.spans.first().map(|(_, e)| *e).unwrap_or(0)
    }

    #[pyo3(signature = (_i=0))]
    fn group(&self, py: Python, _i: i32) -> PyResult<String> {
        let idx = _i as usize;
        if let Some((start, end)) = self.spans.get(idx) {
            let text = self.text.bind(py).to_str()?;
            Ok(unsafe { text.get_unchecked(*start..*end) }.to_string())
        } else {
             Err(PyValueError::new_err(format!("Group {} not found", _i)))
        }
    }

    fn groups(&self, py: Python) -> PyResult<Vec<Option<String>>> {
        let text_bind = self.text.bind(py);
        let text = text_bind.to_str()?;
        
        Ok(self.spans.iter().skip(1).map(|(s, e)| {
            Some(unsafe { text.get_unchecked(*s..*e) }.to_string())
        }).collect())
    }

    fn lastindex(&self) -> usize {
        self.spans.len().saturating_sub(1)
    }
}

impl RuMatch {
    pub fn start(&self) -> usize {
        self.spans.first().map(|(s, _)| *s).unwrap_or(0)
    }

    pub fn end(&self) -> usize {
        self.spans.first().map(|(_, e)| *e).unwrap_or(0)
    }

    pub fn group(&self, _i: i32) -> Result<String, AppError> {
        let idx = _i as usize;
        if let Some((start, end)) = self.spans.get(idx) {
            Ok(unsafe { self.text.get_unchecked(*start..*end) }.to_string())
        } else {
             Err(AppError::IndexOutOfBounds(ReError { message: format!("Group {} not found", _i) }))
        }
    }

    pub fn groups(&self, _i: i32) -> Result<Vec<Option<String>>, AppError> {
        Ok(self.spans.iter().skip(1).map(|(s, e)| {
            Some(unsafe { self.text.get_unchecked(*s..*e) }.to_string())
        }).collect())
    }

    pub fn lastindex(&self) -> usize {
        self.spans.len().saturating_sub(1)
    }
}

// --- CONFIGURATION ---

#[pyclass(frozen)]
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ReConfig {
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
    #[pyo3(signature = (case_insensitive=false, ignore_whitespace=false, multiline=false, unicode_mode=false, size_limit=None, dfa_size_limit=10_000_000, backtrack_limit=None))]
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

// --- REGEX STORAGE ---

pub enum ReEngine {
    Std(Regex),
    Fancy(Regex2),
}

impl ReEngine {

    pub fn is_match(&self, text: &str) -> bool {
        match self {
            ReEngine::Std(re) => re.is_match(text),
            ReEngine::Fancy(re) => re.is_match(text).unwrap_or(false),
        }
    }

    pub fn find(&self, text: &str) -> Option<(usize, usize)> {
        match self {
            ReEngine::Std(re) => re.find(text).map(|m| (m.start(), m.end())),
            ReEngine::Fancy(re) => re.find(text).unwrap_or(None).map(|m| (m.start(), m.end())),
        }
    }

    pub fn split(&self, text: &str) -> Vec<String> {
        match self {
            ReEngine::Std(re) => re.split(text).map(|s| s.to_string()).collect(),
            ReEngine::Fancy(re) => {
                re.split(text).filter_map(|res| res.ok().map(|x| x.to_string()))
                .collect()
            }
        }
    }

    pub fn search(&self, text: &str) -> Result<Option<RuMatch>, AppError> {
        let spans = match &self {
            ReEngine::Std(re) => re.captures(text).map(|c| c.iter().map(|m| m.map(|x| (x.start(), x.end())).unwrap_or((0,0))).collect()),
            ReEngine::Fancy(re) => re.captures(text).unwrap_or(None).map(|c| c.iter().map(|m| m.map(|x| (x.start(), x.end())).unwrap_or((0,0))).collect()),
        };

        match spans {
            Some(s) => Ok(Some(RuMatch { text: text.to_string(), spans: s })),
            None => Ok(None)
        }
    }

    fn sub(&self, repl: &str, text: &str) -> Result<String, AppError> {
        Ok(match &self {
            ReEngine::Std(re) => re.replace_all(text, repl).into_owned(),
            ReEngine::Fancy(re) => re.replace_all(text, repl).into_owned(),
        })
    }

    fn escape(text: &str) -> Result<String, AppError> {
        Ok(regex::escape(text))
    }
}

type CacheMap = HashMap<String, ReEngine>;
type ConfigCacheMap = HashMap<(String, ReConfig), ReEngine>;

thread_local! {
    static CACHE: RefCell<CacheMap> = RefCell::new(HashMap::with_capacity(100));
    static CONFIG_CACHE: RefCell<ConfigCacheMap> = RefCell::new(HashMap::with_capacity(10));
}

// --- COMPILATION LOGIC ---

fn is_fancy_regex(pattern: &str) -> bool {
    let bytes = pattern.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        match bytes[i] {
            b'\\' => {
                if i + 1 < len {
                    let next_char = bytes[i + 1];
                    if (b'1'..=b'9').contains(&next_char) {
                        return true;
                    }
                    i += 1; 
                }
            }
            b'(' => {
                if i + 2 < len && bytes[i + 1] == b'?' {
                    let third = bytes[i + 2];
                    match third {
                        b'=' | b'!' => return true,
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

fn create_engine(pattern: &str, config: Option<&ReConfig>) -> PyResult<ReEngine> {
    let is_fancy = is_fancy_regex(pattern);
    
    if is_fancy {
        let mut builder = RegexBuilder2::new(pattern);
        if let Some(cfg) = config {
            builder.multi_line(cfg.multiline)
                   .case_insensitive(cfg.case_insensitive)
                   .ignore_whitespace(cfg.ignore_whitespace)
                   .unicode_mode(cfg.unicode_mode)
                   .delegate_dfa_size_limit(cfg.dfa_size_limit);
            if let Some(bl) = cfg.backtrack_limit { builder.backtrack_limit(bl); }
            if let Some(sl) = cfg.size_limit { builder.delegate_size_limit(sl); }
        }
        let re = builder.build().map_err(|e| PyValueError::new_err(format!("Regex error: {}", e)))?;
        Ok(ReEngine::Fancy(re))
    } else {
        let mut builder = RegexBuilder::new(pattern);
        if let Some(cfg) = config {
             builder.multi_line(cfg.multiline)
                   .case_insensitive(cfg.case_insensitive)
                   .ignore_whitespace(cfg.ignore_whitespace)
                   .unicode(cfg.unicode_mode)
                   .dfa_size_limit(cfg.dfa_size_limit);
             if let Some(sl) = cfg.size_limit { builder.size_limit(sl); }
        }
        let re = builder.build().map_err(|e| PyValueError::new_err(format!("Regex error: {}", e)))?;
        Ok(ReEngine::Std(re))
    }
}

// --- MAIN API ---

#[pyclass(frozen)]
struct Pattern {
    engine: ReEngine,
}

#[pymethods]
impl Pattern {
    pub fn is_match(&self, text: &Bound<'_, PyString>) -> PyResult<bool> {
        let text_slice = text.to_str()?;
        Ok(self.engine.is_match(text_slice))
    }

    #[pyo3(name = "match")]
    pub fn find(&self, text: &Bound<'_, PyString>) -> PyResult<Option<Match>> {
        let text_slice = text.to_str()?;
        
        let spans = match &self.engine {
            ReEngine::Std(re) => re.find(text_slice).map(|m| smallvec![(m.start(), m.end());1]),
            ReEngine::Fancy(re) => re.find(text_slice).unwrap_or(None).map(|m| smallvec![(m.start(), m.end());1]),
        };

        match spans {
            Some(s) => Ok(Some(Match { text: text.clone().unbind(), spans: s })),
            None => Ok(None)
        }
    }

    fn find_indices(&self, text: &Bound<'_, PyString>) -> PyResult<Option<(usize, usize)>> {
        let text_slice = text.to_str()?;
        Ok(self.engine.find(text_slice))
    }

    pub fn search(&self, text: &Bound<'_, PyString>) -> PyResult<Option<Match>> {
        let text_slice = text.to_str()?;
        // return self.engine.search(text_slice)?;
        let spans = match &self.engine {
            ReEngine::Std(re) => re.captures(text_slice).map(|c| c.iter().map(|m| m.map(|x| (x.start(), x.end())).unwrap_or((0,0))).collect()),
            ReEngine::Fancy(re) => re.captures(text_slice).unwrap_or(None).map(|c| c.iter().map(|m| m.map(|x| (x.start(), x.end())).unwrap_or((0,0))).collect()),
        };

        match spans {
            Some(s) => Ok(Some(Match { text: text.clone().unbind(), spans: s })),
            None => Ok(None)
        }
    }

    pub fn sub(&self, repl: &str, text: &Bound<'_, PyString>) -> PyResult<String> {
        let text_slice = text.to_str()?;
        Ok(self.engine.sub(repl, text_slice)?)
    }
}

#[pyclass]
struct ReRu {}

#[pymethods]
impl ReRu {

    #[staticmethod]
    #[pyo3(signature = (pattern, config=None))]
    pub fn compile(pattern: &str, config: Option<ReConfig>) -> PyResult<Pattern> {
        let engine = create_engine(pattern, config.as_ref())?;
        
        Ok(Pattern { engine })
    }

    #[staticmethod]
    #[pyo3(signature = (pattern, text, config=None))]
    pub fn is_match(pattern: &str, text: &str, config: Option<ReConfig>) -> PyResult<bool> {
        if config.is_none() {
            return CACHE.with(|c| {
                let mut map = c.borrow_mut();
                if let Some(engine) = map.get(pattern) {
                    return Ok(match engine {
                        ReEngine::Std(re) => re.is_match(text),
                        ReEngine::Fancy(re) => re.is_match(text).unwrap_or(false),
                    });
                }
                
                let engine = create_engine(pattern, None)?;
                let res = match &engine {
                    ReEngine::Std(re) => re.is_match(text),
                    ReEngine::Fancy(re) => re.is_match(text).unwrap_or(false),
                };
                map.insert(pattern.to_string(), engine);
                Ok(res)
            });
        }

        let cfg = config.unwrap();
        CONFIG_CACHE.with(|c| {
            let mut map = c.borrow_mut();
            let key = (pattern.to_string(), cfg.clone());
            if let Some(engine) = map.get(&key) {
                return Ok(match engine {
                    ReEngine::Std(re) => re.is_match(text),
                    ReEngine::Fancy(re) => re.is_match(text).unwrap_or(false),
                });
            }
            let engine = create_engine(pattern, Some(&cfg))?;
            let res = match &engine {
                    ReEngine::Std(re) => re.is_match(text),
                    ReEngine::Fancy(re) => re.is_match(text).unwrap_or(false),
            };
            map.insert(key, engine);
            Ok(res)
        })
    }

    #[staticmethod]
    #[pyo3(name = "match", signature = (pattern, text, config=None))]
    pub fn find(pattern: &str, text: &Bound<'_, PyString>, config: Option<ReConfig>) -> PyResult<Option<Match>> {
        let text_slice = text.to_str()?;
        
        let spans = match config {
            None => {
                CACHE.with(|c| {
                    let mut map = c.borrow_mut();
                    // FIX: explicitly return PyResult
                    if let Some(engine) = map.get(pattern) {
                        let res = match engine {
                            ReEngine::Std(re) => re.find(text_slice).map(|m| smallvec![(m.start(), m.end());1]),
                            ReEngine::Fancy(re) => re.find(text_slice).unwrap_or(None).map(|m| smallvec![(m.start(), m.end());1]),
                        };
                        return Ok::<_, PyErr>(res);
                    }
                    let engine = create_engine(pattern, None)?;
                    let res = match &engine {
                        ReEngine::Std(re) => re.find(text_slice).map(|m| smallvec![(m.start(), m.end());1]),
                        ReEngine::Fancy(re) => re.find(text_slice).unwrap_or(None).map(|m| smallvec![(m.start(), m.end());1]),
                    };
                    map.insert(pattern.to_string(), engine);
                    Ok::<_, PyErr>(res)
                })?
            }, Some(cfg) => {
                CONFIG_CACHE.with(|c| {
                    let mut map = c.borrow_mut();
                    let key = (pattern.to_string(), cfg.clone());
                    if let Some(engine) = map.get(&key) {
                        let res = match engine {
                            ReEngine::Std(re) => re.find(text_slice).map(|m| smallvec![(m.start(), m.end());1]),
                            ReEngine::Fancy(re) => re.find(text_slice).unwrap_or(None).map(|m| smallvec![(m.start(), m.end());1]),
                        };
                        return Ok::<_, PyErr>(res);
                    }
                    let engine = create_engine(pattern, Some(&cfg))?;
                    let res = match &engine {
                        ReEngine::Std(re) => re.find(text_slice).map(|m| smallvec![(m.start(), m.end());1]),
                        ReEngine::Fancy(re) => re.find(text_slice).unwrap_or(None).map(|m| smallvec![(m.start(), m.end());1]),
                    };
                    map.insert(key, engine);
                    Ok::<_, PyErr>(res)
                })?
            }
        };

        match spans {
            Some(s) => Ok(Some(Match { text: text.clone().unbind(), spans: s })),
            None => Ok(None)
        }
    }

    #[staticmethod]
    #[pyo3(signature = (pattern, text, config=None))]
    pub fn search(pattern: &str, text: &Bound<'_, PyString>, config: Option<ReConfig>) -> PyResult<Option<Match>> {
        let text_slice = text.to_str()?;
        
        let spans = match config {
            None => {
                CACHE.with(|c| {
                    let mut map = c.borrow_mut();
                    if let Some(engine) = map.get(pattern) {
                        let res = match engine {
                            ReEngine::Std(re) => re.captures(text_slice).map(|c| c.iter().map(|m| m.map(|x| (x.start(), x.end())).unwrap_or((0,0))).collect()),
                            ReEngine::Fancy(re) => re.captures(text_slice).unwrap_or(None).map(|c| c.iter().map(|m| m.map(|x| (x.start(), x.end())).unwrap_or((0,0))).collect()),
                        };
                        return Ok::<_, PyErr>(res);
                    }
                    let engine = create_engine(pattern, None)?;
                    let res = match &engine {
                        ReEngine::Std(re) => re.captures(text_slice).map(|c| c.iter().map(|m| m.map(|x| (x.start(), x.end())).unwrap_or((0,0))).collect()),
                        ReEngine::Fancy(re) => re.captures(text_slice).unwrap_or(None).map(|c| c.iter().map(|m| m.map(|x| (x.start(), x.end())).unwrap_or((0,0))).collect()),
                    };
                    map.insert(pattern.to_string(), engine);
                    Ok::<_, PyErr>(res)
                })?
            },
        Some(cfg) => {
            CONFIG_CACHE.with(|c| {
                let mut map = c.borrow_mut();
                let key = (pattern.to_string(), cfg.clone());
                if let Some(engine) = map.get(&key) {
                     let res = match engine {
                        ReEngine::Std(re) => re.captures(text_slice).map(|c| c.iter().map(|m| m.map(|x| (x.start(), x.end())).unwrap_or((0,0))).collect()),
                        ReEngine::Fancy(re) => re.captures(text_slice).unwrap_or(None).map(|c| c.iter().map(|m| m.map(|x| (x.start(), x.end())).unwrap_or((0,0))).collect()),
                    };
                    return Ok::<_, PyErr>(res);
                }
                let engine = create_engine(pattern, Some(&cfg))?;
                 let res = match &engine {
                        ReEngine::Std(re) => re.captures(text_slice).map(|c| c.iter().map(|m| m.map(|x| (x.start(), x.end())).unwrap_or((0,0))).collect()),
                        ReEngine::Fancy(re) => re.captures(text_slice).unwrap_or(None).map(|c| c.iter().map(|m| m.map(|x| (x.start(), x.end())).unwrap_or((0,0))).collect()),
                };
                map.insert(key, engine);
                Ok::<_, PyErr>(res)
            })?
            }
        };

        match spans {
            Some(s) => Ok(Some(Match { text: text.clone().unbind(), spans: s })),
            None => Ok(None)
        }
    }
}

#[pymodule]
fn reru(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Match>()?;
    m.add_class::<ReConfig>()?;
    m.add_class::<ReRu>()?;
    m.add_class::<Pattern>()?;

    Ok(())
}