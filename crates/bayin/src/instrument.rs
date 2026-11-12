//! What an instrument is: a physical model that turns actions into sound, and the sheet that
//! describes it.

/// The physical model. A [`crate::Player`] drives it: `control` a few hundred times a second
/// with the current parameter values, `sample` once per sample of sound.
pub trait Instrument: Send {
    /// The parameters as the instrument feels them now (smoothed, indexed like [`Spec::params`]),
    /// the gate (0..1: breath, steam, the bow's pressure), and the seconds since the last call.
    /// The next [`BLOCK`] samples follow.
    fn control(&mut self, p: &[f64], gate: f64, dt: f64);
    /// Take the shape just asked for at once instead of easing into it (called once, before any
    /// sound).
    fn settle(&mut self) {}
    /// One sample of radiated sound, in the model's own units ([`Spec::gain`] scales it).
    fn sample(&mut self) -> f64;
}

/// Samples between two `control` calls.
pub const BLOCK: usize = 64;

/// How an instrument makes sound, after Hornbostel and Sachs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    /// a column of air: horns, whistles, pipes, voices
    Aerophone,
    /// a string
    Chordophone,
    /// a stretched skin
    Membranophone,
    /// the body itself: bells, bars, rails
    Idiophone,
}

/// What a parameter is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// what the player does while playing
    Action,
    /// what the instrument is: change it and it is another instrument
    Design,
    /// where it is heard
    Space,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: Kind,
    pub min: f64,
    pub max: f64,
    pub default: f64,
    pub unit: &'static str,
    /// smoothed as a ratio, as pitches are heard
    pub log: bool,
    /// time the instrument takes to feel a change (s)
    pub smooth: f64,
}

impl Param {
    pub const fn new(key: &'static str, label: &'static str, kind: Kind, min: f64, max: f64, default: f64) -> Param {
        Param { key, label, kind, min, max, default, unit: "", log: false, smooth: 0.03 }
    }
    pub const fn unit(mut self, unit: &'static str) -> Param {
        self.unit = unit;
        self
    }
    pub const fn log(mut self) -> Param {
        self.log = true;
        self
    }
    pub const fn smooth(mut self, seconds: f64) -> Param {
        self.smooth = seconds;
        self
    }
}

/// How a phrase moves from the key before to this one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Curve {
    Line,
    /// evenly as a ratio (for pitches)
    Ratio,
    /// hold the old value, then jump
    Step,
}

/// A moment in a phrase: at `t` seconds, these values. `"gate"` is the gate; any other name is a
/// parameter's key.
#[derive(Clone, Debug)]
pub struct Key {
    pub t: f64,
    pub set: Vec<(&'static str, f64)>,
    pub curve: Curve,
}

impl Key {
    pub fn at(t: f64, set: &[(&'static str, f64)]) -> Key {
        Key { t, set: set.to_vec(), curve: Curve::Line }
    }
    pub fn curve(mut self, curve: Curve) -> Key {
        self.curve = curve;
        self
    }
}

/// Something the instrument can play by itself: a call, a signal, a fanfare.
#[derive(Clone, Debug)]
pub struct Phrase {
    pub id: &'static str,
    pub name: &'static str,
    pub keys: Vec<Key>,
}

/// The real recording an instrument was measured against.
#[derive(Clone, Copy, Debug)]
pub struct Reference {
    pub title: &'static str,
    pub url: &'static str,
    pub license: &'static str,
    /// what was matched, and how close it is now
    pub fit: &'static str,
}

/// An instrument's sheet: what it is, what can be done to it, and how to build one.
pub struct Spec {
    pub id: &'static str,
    pub name: &'static str,
    pub family: Family,
    /// how it makes sound, one sentence
    pub blurb: &'static str,
    pub params: Vec<Param>,
    pub phrases: Vec<Phrase>,
    /// seconds for the gate to open and to close
    pub gate: (f64, f64),
    /// scales the model's units to a full scale of ±1
    pub gain: f64,
    pub reference: Option<Reference>,
    /// a new instrument at this sample rate
    pub build: fn(f64) -> Box<dyn Instrument>,
}

impl Spec {
    pub fn param(&self, key: &str) -> Option<usize> {
        self.params.iter().position(|p| p.key == key)
    }
    pub fn phrase(&self, id: &str) -> Option<usize> {
        self.phrases.iter().position(|p| p.id == id)
    }
    /// The default value of every parameter, in order.
    pub fn defaults(&self) -> Vec<f64> {
        self.params.iter().map(|p| p.default).collect()
    }
}
