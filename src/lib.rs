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

#[derive(FromPyObject)]
enum GroupId {
    Index(usize),
    Name(String),
}

#[pymethods]
impl Match {
    fn start(&self) -> usize {
        self.spans.first().map(|(s, _)| *s).unwrap_or(0)
    }

    fn end(&self) -> usize {
        self.spans.first().map(|(_, e)| *e).unwrap_or(0)
    }

    #[pyo3(signature = (ident=GroupId::Index(0)))]
    fn group(&self, py: Python, ident: GroupId) -> PyResult<String> {
        let idx = match ident {
            GroupId::Index(i) => i,
            GroupId::Name(name) => *self.group_map.get(&name).ok_or_else(|| {
                PyValueError::new_err(format!("Group name '{}' not defined", name))
            })?
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

    pub fn engine_info(&self) -> String {
        match &self.inner {
            EngineImpl::Std(_) => "regex".to_string(),
            EngineImpl::Fancy(_) => "fancy_regex".to_string(),
        }
    }

    #[inline]
    pub fn escape(text: &str) -> Result<String, AppError> {
        Ok(regex::escape(text))
    }
}

struct CachedPattern {
    pub engine: Arc<ReEngine>,
    pub match_engine: Arc<ReEngine>,
}

type CacheMap = DashMap<String, Arc<CachedPattern>>;
type ConfigCacheMap = DashMap<(String, ReConfig), Arc<CachedPattern>>;

static CACHE: Lazy<CacheMap> = Lazy::new(|| DashMap::with_capacity(100));
static CONFIG_CACHE: Lazy<ConfigCacheMap> = Lazy::new(|| DashMap::with_capacity(10));

#[pyclass]
#[derive(Debug, Clone, Copy)]
pub enum SelectEngine{
    Std = 0,
    Fancy = 1,
}

fn std_engine(pattern: &str, config: Option<&ReConfig>) -> Result<ReEngine, AppError> {
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
    return Err(AppError::RegexError(ReError { message: "Failed to build regex with 'regex' engine.".to_string()}));
}

fn fancy_engine(pattern: &str, config: Option<&ReConfig>) -> Result<ReEngine, AppError> {
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

fn create_engine(pattern: &str, config: Option<&ReConfig>, engine: Option<SelectEngine>) -> Result<ReEngine, AppError> {
    match engine {
        None => {
            match std_engine(pattern, config) {
                Ok(re_engine) => Ok(re_engine),
                Err(_) => fancy_engine(pattern, config)
            }
        },
        Some(SelectEngine::Std) => {
            std_engine(pattern, config)
        },
        Some(SelectEngine::Fancy) => {
            fancy_engine(pattern, config)
        },
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
    pub fn engine_info(&self) -> String {
        self.engine.engine_info()
    }

    pub fn group_names(&self) -> Vec<String> {
        let names: Vec<String> = self.engine.group_map.iter().map(|entry| entry.key().clone()).collect();
        names
    }

    pub fn is_search(&self, text: &Bound<'_, PyString>) -> PyResult<bool> {
        let text_slice = text.to_str()?;
        Ok(self.engine.is_search(text_slice))
    }

    pub fn is_match(&self, text: &Bound<'_, PyString>) -> PyResult<bool> {
        let text_slice = text.to_str()?;
        Ok(self.match_engine.is_search(text_slice))
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
    let mut char_iter = pattern.chars();
    match char_iter.next() {
        Some('^') => true,
        Some('\\') => {
            match char_iter.next() {
                Some('A') => true,
                _ => false,
            }
        },
        _ => false,
    } 
}


#[pyfunction]
#[pyo3(signature = (pattern, config=None))]
pub fn compile(pattern: &str, config: Option<ReConfig>) -> Result<Pattern, AppError> {
    if config.is_none() {
        if let Some(entry) = CACHE.get(pattern) {
            let cached = entry.value();
            return Ok(Pattern {
                engine: cached.engine.clone(),
                match_engine: cached.match_engine.clone(),
            });
        }
    } else  {
        let cfg = config.unwrap();
        let key = (pattern.to_string(), cfg); 
        if let Some(entry) = CONFIG_CACHE.get(&key) {
            let cached = entry.value();
            return Ok(Pattern {
                engine: cached.engine.clone(),
                match_engine: cached.match_engine.clone(),
            });
        }
    }
    let has_anchored_start = has_match(pattern);
    
    let engine = Arc::new(create_engine(pattern, config.as_ref(), None)?);
    let match_engine = if has_anchored_start {
        engine.clone()
    } else {
        let modified_pattern = format!("^(?:{})", pattern);
        Arc::new(create_engine(&modified_pattern, config.as_ref(), None)?)
    };

    let cached_entry = Arc::new(CachedPattern {
        engine: engine.clone(),
        match_engine: match_engine.clone(),
    });

    if let Some(cfg) = config {
        CONFIG_CACHE.insert((pattern.to_string(), cfg), cached_entry);
    } else {
        CACHE.insert(pattern.to_string(), cached_entry);
    }

    Ok(Pattern { engine, match_engine })
}

#[pyfunction]
#[pyo3(signature = (pattern, config=None, select_engine=None))]
pub fn compile_custom(pattern: &str, config: Option<ReConfig>, select_engine: Option<SelectEngine>) -> Result<Pattern, AppError> {
    let has_match = has_match(pattern);
    match (config, has_match) {
        (None, true) => {
            let engine = Arc::new(create_engine(&pattern, None, select_engine)?);
            Ok(Pattern { engine: Arc::clone(&engine), match_engine: engine })
        },
        (None, false) => {
            let engine = Arc::new(create_engine(&pattern, None, select_engine)?);
            let modified_pattern = format!("^(?:{})", pattern);
            let match_engine = Arc::new(create_engine(&modified_pattern, None, select_engine)?);
            Ok(Pattern { engine, match_engine })
        },
        (Some(cfg), true) => {
            let engine = Arc::new(create_engine(&pattern, Some(&cfg), select_engine)?);
            Ok(Pattern { engine: Arc::clone(&engine), match_engine: engine })
        },
        (Some(cfg), false) => {
            let engine = Arc::new(create_engine(&pattern, Some(&cfg), select_engine)?);
            let modified_pattern = format!("^(?:{})", pattern);
            let match_engine = Arc::new(create_engine(&modified_pattern, Some(&cfg), select_engine)?);
            Ok(Pattern { engine, match_engine })
        }
    }
}

#[pyfunction]
#[pyo3(signature = (pattern, text, config=None))]
pub fn is_match(pattern: &str, text: &Bound<'_, PyString>, config: Option<ReConfig>) -> PyResult<bool> {
    let pattern = compile(pattern, config)?;
    pattern.is_match(text)
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
    m.add_class::<SelectEngine>()?;
    m.add_function(wrap_pyfunction!(compile, m)?)?;
    m.add_function(wrap_pyfunction!(is_match, m)?)?;
    m.add_function(wrap_pyfunction!(is_search, m)?)?;
    m.add_function(wrap_pyfunction!(find, m)?)?;
    m.add_function(wrap_pyfunction!(search, m)?)?;
    m.add_function(wrap_pyfunction!(sub, m)?)?;
    m.add_function(wrap_pyfunction!(escape, m)?)?;
    Ok(())
}