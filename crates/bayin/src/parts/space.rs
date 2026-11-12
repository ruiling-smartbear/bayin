//! The space an instrument is heard in. The airway ends at the instrument's opening; this is the
//! simplest account of what happens after it: distinct echoes off far surfaces (each duller than
//! the last), a diffuse tail, and the air itself taking the top off a distant sound.
//!
//! It is a propagation model, not a simulated place: a handful of delays with losses. Everything
//! is allocated when it is built; `next` allocates nothing.

use crate::air::math::{PI, exp, pow};

struct Echo {
    delay: usize,
    gain: f64,
    coef: f64,
    lp: f64,
}

struct Comb {
    buf: Vec<f64>,
    pos: usize,
    feedback: f64,
    damp: f64,
    lp: f64,
}

struct Allpass {
    buf: Vec<f64>,
    pos: usize,
}

pub struct Space {
    line: Vec<f64>,
    pos: usize,
    echoes: Vec<Echo>,
    combs: Vec<Comb>,
    allpasses: Vec<Allpass>,
    /// how much of the diffuse tail is heard
    wet: f64,
    /// the air's low-pass on everything heard (one pole)
    air: f64,
    air_lp: f64,
}

fn one_pole(fs: f64, hz: f64) -> f64 {
    1.0 - exp(-2.0 * PI * hz / fs)
}

impl Space {
    /// `echoes`: (seconds, gain, low-pass Hz). `combs`: delay times (ms) of the diffuse tail,
    /// which dies by 60 dB in `rt60` seconds, its highs faster (`damp` 0..1).
    fn new(fs: f64, echoes: &[(f64, f64, f64)], combs: &[f64], rt60: f64, damp: f64, allpasses: &[f64], wet: f64, air_hz: f64) -> Space {
        let longest = echoes.iter().map(|e| e.0).fold(0.0, f64::max);
        let n = (longest * fs) as usize + 2;
        Space {
            line: vec![0.0; n],
            pos: 0,
            echoes: echoes.iter().map(|&(t, gain, hz)| Echo { delay: (t * fs) as usize, gain, coef: one_pole(fs, hz), lp: 0.0 }).collect(),
            combs: combs
                .iter()
                .map(|&ms| {
                    let len = ((ms * 1e-3 * fs) as usize).max(1);
                    Comb { buf: vec![0.0; len], pos: 0, feedback: pow(10.0, -3.0 * (ms * 1e-3) / rt60), damp, lp: 0.0 }
                })
                .collect(),
            allpasses: allpasses.iter().map(|&ms| Allpass { buf: vec![0.0; ((ms * 1e-3 * fs) as usize).max(1)], pos: 0 }).collect(),
            wet,
            air: one_pole(fs, air_hz),
            air_lp: 0.0,
        }
    }

    /// Open country, heard from a few hundred metres: echoes off hills and buildings, a long thin
    /// tail, and the air's loss of highs.
    pub fn outdoors(fs: f64) -> Space {
        Space::new(fs, &[(0.23, 0.22, 2400.0), (0.47, 0.14, 1600.0), (0.81, 0.09, 1100.0), (1.27, 0.05, 800.0)], &[61.3, 73.9, 89.1, 107.3], 1.7, 0.5, &[11.3, 3.7], 0.10, 6500.0)
    }

    /// A hall, heard from the seats.
    pub fn hall(fs: f64) -> Space {
        Space::new(fs, &[(0.021, 0.30, 5000.0), (0.043, 0.20, 3500.0)], &[29.7, 37.1, 41.1, 43.7], 1.9, 0.3, &[5.0, 1.7], 0.16, 11000.0)
    }

    #[inline]
    pub fn next(&mut self, x: f64) -> f64 {
        self.next_mix(x, 1.0)
    }

    /// `room` 0..1: how much of the space is heard (0 is the sound at the instrument, dry).
    #[inline]
    pub fn next_mix(&mut self, x: f64, room: f64) -> f64 {
        let n = self.line.len();
        self.line[self.pos] = x;
        let mut early = 0.0;
        for e in self.echoes.iter_mut() {
            let v = self.line[(self.pos + n - e.delay) % n];
            e.lp += e.coef * (v - e.lp);
            early += e.gain * e.lp;
        }
        self.pos = (self.pos + 1) % n;
        // the tail is fed by the sound and its echoes
        let feed = x + early;
        let mut tail = 0.0;
        for c in self.combs.iter_mut() {
            let y = c.buf[c.pos];
            c.lp = y * (1.0 - c.damp) + c.lp * c.damp;
            c.buf[c.pos] = feed + c.lp * c.feedback;
            c.pos = (c.pos + 1) % c.buf.len();
            tail += y;
        }
        for a in self.allpasses.iter_mut() {
            let b = a.buf[a.pos];
            let y = b - tail;
            a.buf[a.pos] = tail + b * 0.5;
            a.pos = (a.pos + 1) % a.buf.len();
            tail = y;
        }
        let heard = x + room * (early + self.wet * tail);
        self.air_lp += self.air * (heard - self.air_lp);
        x + room * (self.air_lp - x)
    }
}
