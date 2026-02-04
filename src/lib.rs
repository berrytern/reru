use std::hash::Hash;
use std::sync::Arc;
use dashmap::DashMap;
use once_cell::sync::Lazy;
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
    group_map: Arc<DashMap<String, usize>>,
}

pub struct RuMatch {
    text: String, 
    spans: SpanVec,
    group_map: Arc<DashMap<String, usize>>,
}

impl From<RuMatch> for Match {
    fn from(rm: RuMatch) -> Self {
        Python::attach(|py| {
            Match {
                text: PyString::new(py, &rm.text).into(),
                spans: rm.spans,
                group_map: rm.group_map.clone(),
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

    #[pyo3(signature = (ident))]
    fn group(&self, py: Python, ident: &Bound<'_, PyAny>) -> PyResult<String> {
        let idx = if let Ok(i) = ident.extract::<usize>() {
            i
        } else if let Ok(name) = ident.extract::<String>() {
            *self.group_map.get(&name).ok_or_else(|| {
                PyValueError::new_err(format!("Group name '{}' not defined", name))
            })?
        } else {
            return Err(PyValueError::new_err("Group argument must be int or str"));
        };

        if let Some((start, end)) = self.spans.get(idx) {
            let text = self.text.bind(py).to_str()?;
            Ok(unsafe { text.get_unchecked(*start..*end) }.to_string())
        } else {
            Err(PyValueError::new_err(format!("Group {} not found", idx)))
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
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
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
#[derive(Debug, Clone)]
pub struct  ReEngine {
    inner: EngineImpl,
    group_map: Arc<DashMap<String, usize>>,
}

#[derive(Debug, Clone)]
pub enum EngineImpl {
    Std(Regex),
    Fancy(Regex2),
}

impl ReEngine {

    #[inline]
    pub fn is_search(&self, text: &str) -> bool {
        match &self.inner {
            EngineImpl::Std(re) => re.is_match(text),
            EngineImpl::Fancy(re) => re.is_match(text).unwrap_or(false),
        }
    }

    #[inline]
    pub fn find(&self, text: &str) -> Option<(usize, usize)> {
        match &self.inner {
            EngineImpl::Std(re) => re.find(text).map(|m| (m.start(), m.end())),
            EngineImpl::Fancy(re) => re.find(text).unwrap_or(None).map(|m| (m.start(), m.end())),
        }
    }

    #[inline]
    pub fn fmatch(&self, text: &str) -> Option<RuMatch> {
        match &self.inner {
            EngineImpl::Std(re) => re.captures(text).and_then(|captures| {
                let mat = captures.get(0).unwrap();
                if mat.start() == 0 {
                    let s = captures.iter().map(|m| m.map(|x| (x.start(), x.end())).unwrap_or((0,0))).collect();
                    Some(RuMatch { text: text.to_string(), spans: s, group_map: self.group_map.clone() })
                } else {
                    None
                }
            }),
            EngineImpl::Fancy(re) => re.captures(text).unwrap_or(None).and_then(|captures| {
                let mat = captures.get(0).unwrap();
                if mat.start() == 0 {
                    let s = captures.iter().map(|m| m.map(|x| (x.start(), x.end())).unwrap_or((0,0))).collect();
                    Some(RuMatch { text: text.to_string(), spans: s, group_map: self.group_map.clone() })
                } else {
                    None
                }
            }),
        }
    }

    #[inline]
    pub fn split(&self, text: &str) -> Vec<String> {
        match &self.inner {
            EngineImpl::Std(re) => re.split(text).map(|s| s.to_string()).collect(),
            EngineImpl::Fancy(re) => {
                re.split(text).filter_map(|res| res.ok().map(|x| x.to_string()))
                .collect()
            }
        }
    }

    #[inline]
    pub fn search(&self, text: &str) -> Result<Option<RuMatch>, AppError> {
        let spans = match &self.inner {
            EngineImpl::Std(re) => re.captures(text).map(|c| c.iter().map(|m| m.map(|x| (x.start(), x.end())).unwrap_or((0,0))).collect()),
            EngineImpl::Fancy(re) => re.captures(text).unwrap_or(None).map(|c| c.iter().map(|m| m.map(|x| (x.start(), x.end())).unwrap_or((0,0))).collect()),
        };

        match spans {
            Some(s) => Ok(Some(RuMatch { text: text.to_string(), spans: s, group_map: self.group_map.clone() })),
            None => Ok(None)
        }
    }

    #[inline]
    pub fn sub(&self, repl: &str, text: &str) -> Result<String, AppError> {
        Ok(match &self.inner {
            EngineImpl::Std(re) => re.replace_all(text, repl).into_owned(),
            EngineImpl::Fancy(re) => re.replace_all(text, repl).into_owned(),
        })
    }

    #[inline]
    pub fn escape(text: &str) -> Result<String, AppError> {
        Ok(regex::escape(text))
    }
}

type CacheMap = DashMap<String, Arc<ReEngine>>;
type ConfigCacheMap = DashMap<(String, ReConfig), Arc<ReEngine>>;

static CACHE: Lazy<CacheMap> = Lazy::new(|| DashMap::with_capacity(100));
static CONFIG_CACHE: Lazy<ConfigCacheMap> = Lazy::new(|| DashMap::with_capacity(10));


fn create_engine(pattern: &str, config: Option<&ReConfig>) -> Result<ReEngine, AppError> {
    let mut builder = RegexBuilder::new(pattern);
    if let Some(cfg) = config {
        builder.multi_line(cfg.multiline)
            .case_insensitive(cfg.case_insensitive)
            .ignore_whitespace(cfg.ignore_whitespace)
            .unicode(cfg.unicode_mode)
            .dfa_size_limit(cfg.dfa_size_limit);
        if let Some(sl) = cfg.size_limit { builder.size_limit(sl); }
    }
    if let Ok(re) = builder.build(){
        let names = re.capture_names().map(|n| n.map(|s| s.to_string()));
        let map = DashMap::new();
        for (i, name_opt) in names.into_iter().enumerate() {
            if let Some(name) = name_opt {
                map.insert(name, i);
            }
        }
        return Ok(ReEngine{inner: EngineImpl::Std(re), group_map: Arc::new(map)});
    };
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
    match builder.build() {
        Ok(re) => {
            let names = re.capture_names().map(|n| n.map(|s| s.to_string()));
            let map = DashMap::new();
            for (i, name_opt) in names.into_iter().enumerate() {
                if let Some(name) = name_opt {
                    map.insert(name, i);
                }
            }
            Ok(ReEngine{inner: EngineImpl::Fancy(re), group_map: Arc::new(map)})
        },
        Err(e) => Err(AppError::RegexError(ReError { message: format!("Regex error: {}", e)})),
    }
}

// --- MAIN API ---

#[pyclass(frozen)]
#[derive(Debug,Clone)]
pub struct Pattern {
    engine: Arc<ReEngine>,
    match_engine: Arc<ReEngine>,
}

#[pymethods]
impl Pattern {

    pub fn group_names(&self) -> Vec<String> {
        let names: Vec<String> = self.engine.group_map.iter().map(|entry| entry.key().clone()).collect();
        names
    }


    pub fn is_search(&self, text: &Bound<'_, PyString>) -> PyResult<bool> {
        let text_slice = text.to_str()?;
        Ok(self.engine.is_search(text_slice))
    }

    pub fn find(&self, text: &Bound<'_, PyString>) -> PyResult<Option<Match>> {
        let text_slice = text.to_str()?;
        
        let spans = match &self.engine.inner {
            EngineImpl::Std(re) => re.find(text_slice).map(|m| smallvec![(m.start(), m.end());1]),
            EngineImpl::Fancy(re) => re.find(text_slice).unwrap_or(None).map(|m| smallvec![(m.start(), m.end());1]),
        };

        match spans {
            Some(s) => Ok(Some(Match { text: text.clone().unbind(), spans: s, group_map: self.engine.group_map.clone() })),
            None => Ok(None)
        }
    }

    pub fn findall(&self, text: &str) -> PyResult<Vec<String>> {
        Ok(match &self.engine.inner {
            EngineImpl::Std(re) => re.find_iter(text).map(|mat| mat.as_str().to_string()).collect(),
            EngineImpl::Fancy(re) => re.find_iter(text).filter_map(|res| res.ok().map(|mat| mat.as_str().to_string())).collect(),
        })
    }

    #[pyo3(name = "match")]
    pub fn fmatch(&self, text: &Bound<'_, PyString>) -> PyResult<Option<Match>> {
        let text_slice = text.to_str()?;
        // return self.engine.search(text_slice)?; faster with code replication
        let spans = match &self.match_engine.inner {
            EngineImpl::Std(re) => re.captures(text_slice).map(|c| c.iter().map(|m| m.map(|x| (x.start(), x.end())).unwrap_or((0,0))).collect()),
            EngineImpl::Fancy(re) => re.captures(text_slice).unwrap_or(None).map(|c| c.iter().map(|m| m.map(|x| (x.start(), x.end())).unwrap_or((0,0))).collect()),
        };

        match spans {
            Some(s) => Ok(Some(Match { text: text.clone().unbind(), spans: s, group_map: self.engine.group_map.clone() })),
            None => Ok(None)
        }
    }

    fn find_indices(&self, text: &Bound<'_, PyString>) -> PyResult<Option<(usize, usize)>> {
        let text_slice = text.to_str()?;
        Ok(self.engine.find(text_slice))
    }

    pub fn search(&self, text: &Bound<'_, PyString>) -> PyResult<Option<Match>> {
        let text_slice = text.to_str()?;
        // return self.engine.search(text_slice)?; faster with code replication
        let spans = match &self.engine.inner {
            EngineImpl::Std(re) => re.captures(text_slice).map(|c| c.iter().map(|m| m.map(|x| (x.start(), x.end())).unwrap_or((0,0))).collect()),
            EngineImpl::Fancy(re) => re.captures(text_slice).unwrap_or(None).map(|c| c.iter().map(|m| m.map(|x| (x.start(), x.end())).unwrap_or((0,0))).collect()),
        };

        match spans {
            Some(s) => Ok(Some(Match { text: text.clone().unbind(), spans: s, group_map: self.engine.group_map.clone() })),
            None => Ok(None)
        }
    }

    pub fn sub(&self, repl: &str, text: &Bound<'_, PyString>) -> PyResult<String> {
        let text_slice = text.to_str()?;
        Ok(self.engine.sub(repl, text_slice)?)
    }

    #[staticmethod]
    pub fn escape(text: &Bound<'_, PyString>) -> PyResult<String> {
        let text_slice = text.to_str()?;
        Ok(ReEngine::escape(text_slice)?)
    }
}

fn has_match(pattern: &str) -> bool {
    pattern.starts_with('^') || pattern.starts_with(r"\A")
}


#[pyfunction]
#[pyo3(signature = (pattern, config=None))]
pub fn compile(pattern: &str, config: Option<ReConfig>) -> Result<Pattern, AppError> {
    let has_match = has_match(pattern);
    match config {
        None => {
            return match (has_match, CACHE.get(pattern)) {
                (true, Some(entry)) => {
                    let engine = Arc::clone(entry.value());
                    Ok(Pattern { engine: Arc::clone(&engine), match_engine: engine })
                },
                (false, Some(entry)) => {
                    let engine = Arc::clone(entry.value());
                    let modified_pattern = format!("^(?:{})", pattern);
                    let match_engine = if let Some(entry2) = CACHE.get(&modified_pattern) {
                        Arc::clone(entry2.value())
                    } else {
                        let me = Arc::new(create_engine(&modified_pattern, None)?);
                        CACHE.entry(modified_pattern).or_insert(Arc::clone(&me));
                        me
                    };
                    Ok(Pattern { engine: Arc::clone(&engine), match_engine })
                },
                (true, None) => {
                    let engine = Arc::new(create_engine(&pattern, None)?);
                    Ok(Pattern { engine: Arc::clone(&engine), match_engine: engine })
                },
                (false, None) => {
                    let engine = Arc::new(create_engine(&pattern, None)?);
                    let modified_pattern = format!("^(?:{})", pattern);
                    let match_engine = Arc::new(create_engine(&modified_pattern, None)?);
                    CACHE.entry(pattern.to_string()).or_insert(Arc::clone(&engine));
                    CACHE.entry(modified_pattern).or_insert(Arc::clone(&match_engine));
                    Ok(Pattern { engine, match_engine })
                }
            };
        },
        Some(cfg) => {
            let key = (pattern.to_string(), cfg);
            return match (has_match, CONFIG_CACHE.get(&key)) {
                (true, Some(entry)) => {
                    let engine = Arc::clone(entry.value());
                    Ok(Pattern { engine: Arc::clone(&engine), match_engine: engine })
                },
                (false, Some(entry)) => {
                    let engine = Arc::clone(entry.value());
                    let modified_pattern = format!("^(?:{})", pattern);
                    let match_engine = if let Some(entry2) = CONFIG_CACHE.get(&(modified_pattern.clone(), cfg)) {
                        Arc::clone(entry2.value())
                    } else {
                        let me = Arc::new(create_engine(&modified_pattern, Some(&cfg))?);
                        CONFIG_CACHE.entry((modified_pattern.clone(), cfg)).or_insert(Arc::clone(&me));
                        me
                    };
                    Ok(Pattern { engine: Arc::clone(&engine), match_engine })
                },
                (true, None) => {
                    let engine = Arc::new(create_engine(&pattern, Some(&cfg))?);
                    Ok(Pattern { engine: Arc::clone(&engine), match_engine: engine })
                },
                (false, None) => {
                    let engine = Arc::new(create_engine(&pattern, Some(&cfg))?);
                    let modified_pattern = format!("^(?:{})", pattern);
                    let match_engine = Arc::new(create_engine(&modified_pattern, Some(&cfg))?);
                    CONFIG_CACHE.entry((pattern.to_string(), cfg)).or_insert(Arc::clone(&engine));
                    CONFIG_CACHE.entry((modified_pattern.clone(), cfg)).or_insert(Arc::clone(&match_engine));
                    Ok(Pattern { engine, match_engine })
                }
            };
        }
    }
}

#[pyfunction]
#[pyo3(signature = (pattern, text, config=None))]
pub fn is_match(pattern: &str, text: &Bound<'_, PyString>, config: Option<ReConfig>) -> PyResult<bool> {
    match pattern.chars().next() {
        Some('^') => {
            return is_search(pattern, text, config);
        },
        Some(_) =>  {
            let modified_pattern = format!("^(?:{})", pattern);
            return is_search(&modified_pattern, text, config);
        },
        None => Ok(true)
    }
}

#[pyfunction]
#[pyo3(signature = (pattern, text, config=None))]
pub fn is_search(pattern: &str, text: &Bound<'_, PyString>, config: Option<ReConfig>) -> PyResult<bool> {
    let pattern = compile(pattern, config)?;
    pattern.is_search(text)
}

#[pyfunction]
#[pyo3(name = "match", signature = (pattern, text, config=None))]
pub fn find(pattern: &str, text: &Bound<'_, PyString>, config: Option<ReConfig>) -> PyResult<Option<Match>> {
    let pattern = compile(pattern, config)?;
    pattern.fmatch(text)
}

#[pyfunction]
#[pyo3(signature = (pattern, text, config=None))]
pub fn search(pattern: &str, text: &Bound<'_, PyString>, config: Option<ReConfig>) -> PyResult<Option<Match>> {
    let pattern = compile(pattern, config)?;
    pattern.search(text)
}
#[pyfunction]
#[pyo3(signature = (pattern, repl, text, config=None))]
pub fn sub(pattern: &str, repl: &str, text: &Bound<'_, PyString>, config: Option<ReConfig>) -> PyResult<String> {
    let pattern = compile(pattern, config)?;
    pattern.sub(repl, text)
}
#[pyfunction]
#[pyo3(signature = (text))]
pub fn escape(text: &Bound<'_, PyString>) -> PyResult<String> {
    Ok(Pattern::escape(text)?)
}

#[pymodule]
fn reru(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Match>()?;
    m.add_class::<ReConfig>()?;
    m.add_class::<Pattern>()?;
    m.add_function(wrap_pyfunction!(compile, m)?)?;
    m.add_function(wrap_pyfunction!(is_match, m)?)?;
    m.add_function(wrap_pyfunction!(is_search, m)?)?;
    m.add_function(wrap_pyfunction!(find, m)?)?;
    m.add_function(wrap_pyfunction!(search, m)?)?;
    m.add_function(wrap_pyfunction!(sub, m)?)?;
    m.add_function(wrap_pyfunction!(escape, m)?)?;
    Ok(())
}