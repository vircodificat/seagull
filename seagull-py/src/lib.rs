use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

// ---------------------------------------------------------------------------
// Key
// ---------------------------------------------------------------------------

#[pyclass(name = "Key", module = "seagull", eq, hash, frozen)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Key {
    LeftS,
    LeftT,
    LeftK,
    LeftP,
    LeftW,
    LeftH,
    LeftR,
    MiddleA,
    MiddleO,
    MiddleStar,
    MiddleE,
    MiddleU,
    RightF,
    RightR,
    RightP,
    RightB,
    RightL,
    RightG,
    RightT,
    RightS,
    RightD,
    RightZ,
}

impl From<seagull::Key> for Key {
    fn from(k: seagull::Key) -> Self {
        match k {
            seagull::Key::LeftS      => Key::LeftS,
            seagull::Key::LeftT      => Key::LeftT,
            seagull::Key::LeftK      => Key::LeftK,
            seagull::Key::LeftP      => Key::LeftP,
            seagull::Key::LeftW      => Key::LeftW,
            seagull::Key::LeftH      => Key::LeftH,
            seagull::Key::LeftR      => Key::LeftR,
            seagull::Key::MiddleA    => Key::MiddleA,
            seagull::Key::MiddleO    => Key::MiddleO,
            seagull::Key::MiddleStar => Key::MiddleStar,
            seagull::Key::MiddleE    => Key::MiddleE,
            seagull::Key::MiddleU    => Key::MiddleU,
            seagull::Key::RightF     => Key::RightF,
            seagull::Key::RightR     => Key::RightR,
            seagull::Key::RightP     => Key::RightP,
            seagull::Key::RightB     => Key::RightB,
            seagull::Key::RightL     => Key::RightL,
            seagull::Key::RightG     => Key::RightG,
            seagull::Key::RightT     => Key::RightT,
            seagull::Key::RightS     => Key::RightS,
            seagull::Key::RightD     => Key::RightD,
            seagull::Key::RightZ     => Key::RightZ,
        }
    }
}

#[pymethods]
impl Key {
    fn __str__(&self) -> &'static str {
        match self {
            Key::LeftS      => "S",
            Key::LeftT      => "T",
            Key::LeftK      => "K",
            Key::LeftP      => "P",
            Key::LeftW      => "W",
            Key::LeftH      => "H",
            Key::LeftR      => "R",
            Key::MiddleA    => "A",
            Key::MiddleO    => "O",
            Key::MiddleStar => "*",
            Key::MiddleE    => "E",
            Key::MiddleU    => "U",
            Key::RightF     => "F",
            Key::RightR     => "R",
            Key::RightP     => "P",
            Key::RightB     => "B",
            Key::RightL     => "L",
            Key::RightG     => "G",
            Key::RightT     => "T",
            Key::RightS     => "S",
            Key::RightD     => "D",
            Key::RightZ     => "Z",
        }
    }
}

// ---------------------------------------------------------------------------
// Iterators
// ---------------------------------------------------------------------------

#[pyclass]
struct StrokeIter { strokes: Vec<Stroke>, index: usize }

#[pymethods]
impl StrokeIter {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> { slf }
    fn __next__(mut slf: PyRefMut<Self>) -> Option<Stroke> {
        if slf.index < slf.strokes.len() {
            let s = slf.strokes[slf.index].clone();
            slf.index += 1;
            Some(s)
        } else {
            None
        }
    }
}

#[pyclass]
struct KeyIter { keys: Vec<Key>, index: usize }

#[pymethods]
impl KeyIter {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> { slf }
    fn __next__(mut slf: PyRefMut<Self>) -> Option<Key> {
        if slf.index < slf.keys.len() {
            let k = slf.keys[slf.index];
            slf.index += 1;
            Some(k)
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Stroke
// ---------------------------------------------------------------------------

#[pyclass(name = "Stroke", module = "seagull", frozen, eq, hash)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Stroke(seagull::Stroke);

#[pymethods]
impl Stroke {
    #[new]
    fn new(s: &str) -> PyResult<Self> {
        seagull::Stroke::try_from_string(s)
            .map(Stroke)
            .ok_or_else(|| PyValueError::new_err(format!("Invalid stroke '{s}'")))
    }

    fn __repr__(&self) -> String {
        format!("Stroke('{}')", self.0)
    }

    fn __str__(&self) -> String {
        format!("{}", self.0)
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<KeyIter>> {
        Py::new(slf.py(), KeyIter { keys: slf.0.keys().into_iter().map(Key::from).collect(), index: 0 })
    }

    /// Return the individual keys that make up this stroke.
    fn keys(&self) -> Vec<Key> {
        self.0.keys().into_iter().map(Key::from).collect()
    }

    fn initials(&self) -> Stroke { Stroke(self.0.initials()) }
    fn middles(&self)  -> Stroke { Stroke(self.0.middles()) }
    fn finals(&self)   -> Stroke { Stroke(self.0.finals()) }
}

// ---------------------------------------------------------------------------
// Outline
// ---------------------------------------------------------------------------

#[pyclass(name = "Outline", module = "seagull", frozen, eq, hash)]
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Outline(seagull::Outline);

#[pymethods]
impl Outline {
    #[new]
    fn new(s: &str) -> PyResult<Self> {
        seagull::Outline::try_from_string(s)
            .map(Outline)
            .ok_or_else(|| PyValueError::new_err(format!("Invalid outline '{s}'")))
    }

    fn __repr__(&self) -> String {
        format!("Outline('{}')", self.0)
    }

    fn __str__(&self) -> String {
        format!("{}", self.0)
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<StrokeIter>> {
        Py::new(slf.py(), StrokeIter { strokes: slf.0.strokes().iter().copied().map(Stroke).collect(), index: 0 })
    }

    /// Return the individual strokes that make up this outline.
    fn strokes(&self) -> Vec<Stroke> {
        self.0.strokes().iter().copied().map(Stroke).collect()
    }
}

// ---------------------------------------------------------------------------
// Module
// ---------------------------------------------------------------------------

#[pymodule(name = "seagull")]
fn module_init(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Key>()?;
    m.add_class::<Stroke>()?;
    m.add_class::<Outline>()?;
    m.add_class::<StrokeIter>()?;
    m.add_class::<KeyIter>()?;
    Ok(())
}
