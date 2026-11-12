//! The player: holds an instrument, eases its parameters toward what is asked, opens and closes
//! the gate, plays phrases, and hands out sound.
//!
//! It allocates when it is made and never while rendering, and the sound does not depend on how
//! the caller slices its `render` calls.

use crate::air::math::{PI, exp, pow, tanh};
use crate::instrument::{BLOCK, Curve, Instrument, Spec};

#[derive(Clone, Copy)]
enum Target {
    Gate,
    Param(usize),
}

struct ActivePhrase {
    index: usize,
    t0: f64,
}

pub struct Player {
    spec: &'static Spec,
    fs: f64,
    instrument: Box<dyn Instrument>,
    /// what the performer set, what is asked for now (the performer or a phrase), and what the
    /// instrument feels (smoothed); each indexed like the spec's parameters
    performer: Vec<f64>,
    asked: Vec<f64>,
    felt: Vec<f64>,
    gate: f64,
    gate_asked: f64,
    phrase_gate: f64,
    clock: f64,
    phrase: Option<ActivePhrase>,
    /// each phrase's keys with their names resolved, and the parameters it moves
    plans: Vec<Vec<(f64, Vec<(Target, f64)>, Curve)>>,
    owns: Vec<Vec<usize>>,
    countdown: usize,
    dc_x: f64,
    dc_y: f64,
    dc_coef: f64,
}

impl Player {
    /// A player of `spec` at sample rate `fs`. `initial` sets parameters by index.
    pub fn new(spec: &'static Spec, fs: f64, initial: &[(usize, f64)]) -> Player {
        let mut performer = spec.defaults();
        for &(i, x) in initial {
            if let Some(p) = spec.params.get(i)
                && x.is_finite()
            {
                performer[i] = x.clamp(p.min, p.max);
            }
        }
        let mut plans = Vec::new();
        let mut owns = Vec::new();
        for ph in &spec.phrases {
            let mut own: Vec<usize> = Vec::new();
            let plan = ph
                .keys
                .iter()
                .map(|k| {
                    let set = k
                        .set
                        .iter()
                        .map(|&(name, x)| {
                            if name == "gate" {
                                (Target::Gate, x)
                            } else {
                                let i = spec.param(name).unwrap_or_else(|| panic!("{}: phrase {} names no parameter {name}", spec.id, ph.id));
                                if !own.contains(&i) {
                                    own.push(i);
                                }
                                (Target::Param(i), x)
                            }
                        })
                        .collect();
                    (k.t, set, k.curve)
                })
                .collect();
            plans.push(plan);
            owns.push(own);
        }
        let mut instrument = (spec.build)(fs);
        instrument.control(&performer, 0.0, BLOCK as f64 / fs);
        instrument.settle();
        Player {
            spec,
            fs,
            instrument,
            asked: performer.clone(),
            felt: performer.clone(),
            performer,
            gate: 0.0,
            gate_asked: 0.0,
            phrase_gate: 0.0,
            clock: 0.0,
            phrase: None,
            plans,
            owns,
            countdown: 0,
            dc_x: 0.0,
            dc_y: 0.0,
            dc_coef: exp((-2.0 * PI * 12.0) / fs),
        }
    }

    pub fn spec(&self) -> &'static Spec {
        self.spec
    }
    /// The parameters as the instrument feels them now.
    pub fn felt(&self) -> &[f64] {
        &self.felt
    }
    /// The gate as the instrument feels it now.
    pub fn gate(&self) -> f64 {
        self.gate
    }
    pub fn clock(&self) -> f64 {
        self.clock
    }

    /// Set parameters by index. One a playing phrase is moving takes the value when the phrase ends.
    pub fn set(&mut self, values: &[(usize, f64)]) {
        for &(i, x) in values {
            let Some(p) = self.spec.params.get(i) else { continue };
            if !x.is_finite() {
                continue;
            }
            let x = x.clamp(p.min, p.max);
            self.performer[i] = x;
            let owned = self.phrase.as_ref().is_some_and(|ph| self.owns[ph.index].contains(&i));
            if !owned {
                self.asked[i] = x;
            }
        }
    }

    /// Ask for the gate (0 closed, 1 open).
    pub fn set_gate(&mut self, level: f64) {
        self.gate_asked = level.clamp(0.0, 1.5);
    }

    /// Start a phrase by index (it takes over the parameters it names until it ends).
    pub fn play(&mut self, phrase: usize) {
        if phrase >= self.plans.len() {
            return;
        }
        self.end_phrase();
        self.phrase = Some(ActivePhrase { index: phrase, t0: self.clock });
    }

    pub fn playing(&self) -> bool {
        self.phrase.is_some()
    }

    fn end_phrase(&mut self) {
        if let Some(ph) = self.phrase.take() {
            for &i in &self.owns[ph.index] {
                self.asked[i] = self.performer[i];
            }
        }
        self.phrase_gate = 0.0;
    }

    /// Where the playing phrase stands at the clock: write what it asks for.
    fn follow_phrase(&mut self) {
        let Some(ph) = &self.phrase else { return };
        let t = self.clock - ph.t0;
        let index = ph.index;
        let keys = &self.plans[index];
        if t > keys[keys.len() - 1].0 + 0.05 {
            self.end_phrase();
            return;
        }
        // every target the phrase names, from the keys around t that set it
        for (k, (_, set, _)) in keys.iter().enumerate() {
            for &(target, _) in set {
                // handle each target once, at the first key that names it
                let first = keys[..k].iter().all(|(_, s, _)| !s.iter().any(|(n, _)| same(*n, target)));
                if !first {
                    continue;
                }
                let mut prev: Option<(f64, f64)> = None;
                let mut next: Option<(f64, f64, Curve)> = None;
                for (kt, s, curve) in keys {
                    let Some(&(_, x)) = s.iter().find(|(n, _)| same(*n, target)) else { continue };
                    if *kt <= t {
                        prev = Some((*kt, x));
                    } else {
                        next = Some((*kt, x, *curve));
                        break;
                    }
                }
                let Some((pt, px)) = prev else { continue };
                let mut x = px;
                if let Some((nt, nx, curve)) = next
                    && curve != Curve::Step
                {
                    let u = (t - pt) / (nt - pt).max(1e-6);
                    x = if curve == Curve::Ratio && px > 0.0 && nx > 0.0 { px * pow(nx / px, u) } else { px + (nx - px) * u };
                }
                match target {
                    Target::Gate => self.phrase_gate = x,
                    Target::Param(i) => {
                        let p = &self.spec.params[i];
                        self.asked[i] = x.clamp(p.min, p.max);
                    }
                }
            }
        }
    }

    fn control(&mut self) {
        let dt = BLOCK as f64 / self.fs;
        self.follow_phrase();
        for (i, p) in self.spec.params.iter().enumerate() {
            let a = 1.0 - exp(-dt / p.smooth.max(0.001));
            let (cur, tgt) = (self.felt[i], self.asked[i]);
            self.felt[i] = if p.log && cur > 0.0 && tgt > 0.0 { cur * pow(tgt / cur, a) } else { cur + (tgt - cur) * a };
        }
        let goal = self.gate_asked.max(self.phrase_gate);
        let tau = if goal > self.gate { self.spec.gate.0 } else { self.spec.gate.1 };
        self.gate += (goal - self.gate) * (1.0 - exp(-dt / tau));
        self.instrument.control(&self.felt, self.gate, dt);
        self.countdown = BLOCK;
        self.clock += dt;
    }

    /// The next `out.len()` samples of sound, within ±1.
    pub fn render(&mut self, out: &mut [f32]) {
        let gain = self.spec.gain;
        for o in out.iter_mut() {
            if self.countdown == 0 {
                self.control();
            }
            self.countdown -= 1;
            let mut v = self.instrument.sample() * gain;
            // take off any steady offset, then round off whatever would pass full scale
            let hp = v - self.dc_x + self.dc_coef * self.dc_y;
            self.dc_x = v;
            self.dc_y = hp;
            v = hp;
            let av = v.abs();
            if av > 0.7 {
                v = v.signum() * (0.7 + 0.3 * tanh((av - 0.7) / 0.3));
            }
            *o = if v.is_finite() { v as f32 } else { 0.0 };
        }
    }
}

fn same(a: Target, b: Target) -> bool {
    match (a, b) {
        (Target::Gate, Target::Gate) => true,
        (Target::Param(i), Target::Param(j)) => i == j,
        _ => false,
    }
}
