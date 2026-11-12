//! Steam whistle: three stopped pipes, each blown across its mouth by a sheet of steam. The jet
//! is an oscillator that the pressure at the pipe's mouth pulls on, so it locks onto its pipe's
//! resonance; a stopped pipe sounds a quarter wavelength, so the bells are short for their pitch.
//!
//! What makes it a steam whistle and not three tones:
//!  - The jet saturates: it can swing no further than fully into the bell or fully out, so the
//!    flow it drives is flattened, with odd harmonics, which a stopped pipe carries.
//!  - Steam is turbulent: each jet's strength wanders, so the notes waver against each other.
//!  - Steam hisses: part of the noise goes into the bell and comes out coloured by it, part
//!    escapes straight to the air.
//!  - More steam is louder, hissier and sharper, and as the valve opens the notes scoop up from
//!    below: the wail of a cord pulled slowly.
//!
//! It has not yet been fitted to a recording of a real whistle.

use crate::air::math::{PI, Rng, exp, sqrt, tanh};
use crate::air::{AirwayNetwork, C_SOUND, Tube, TubeArea, TubeOpts, Whistle, tube};
use crate::instrument::{BLOCK, Family, Instrument, Key, Kind, Param, Phrase, Spec};
use crate::parts::space::Space;

const STEAM: usize = 0;
const CHIMES: usize = 1;
const ROOM: usize = 2;

/// The three bells' notes (Hz): a minor triad in its second inversion.
const NOTES: [f64; 3] = [311.13, 392.00, 466.16];
/// Bell area (cm²).
const BELL: f64 = 18.0;
/// The open mouth's end correction (cm): the air column is this much shorter than a quarter
/// wavelength of its note (measured from renders: 2.1–2.4 cm for this bell area).
const END_CORRECTION: f64 = 2.55;
/// The jet's volume flow at full swing (cm³/s).
const JET_FLOW: f64 = 400.0;
/// Hiss into the bell (cm³/s) and straight to the air (in the units of the radiated sound).
const HISS_IN: f64 = 140.0;
const HISS_OUT: f64 = 2.2e6;

struct Bell {
    net: AirwayNetwork,
    pipe: Tube,
    jet: Whistle,
    note: f64,
    on: bool,
    /// this jet's slow wander (−1..1)
    wander: f64,
}

struct SteamWhistle {
    bells: Vec<Bell>,
    rng: Rng,
    space: Space,
    room: f64,
    /// 0..1, the steam through the valve
    drive: f64,
    /// one-pole coefficients: the wander's speed, and the hiss's band
    wander_coef: f64,
    hiss_hp: f64,
    hiss_lp: f64,
    hp_state: f64,
    lp_state: f64,
}

fn bell(fs: f64, note: f64) -> Bell {
    let mut net = AirwayNetwork::new(fs);
    let length = C_SOUND / (4.0 * note) - END_CORRECTION;
    // the mouth is the first node (it radiates); the far end is the closed top of the bell
    let pipe = tube(&mut net, "bell", fs, length, TubeArea::Uniform(BELL), TubeOpts::default());
    net.add_radiation(pipe.first_node());
    net.finalize();
    Bell { net, pipe, jet: Whistle::new(fs), note, on: false, wander: 0.0 }
}

fn one_pole(fs: f64, hz: f64) -> f64 {
    1.0 - exp(-2.0 * PI * hz / fs)
}

fn build(fs: f64) -> Box<dyn Instrument> {
    Box::new(SteamWhistle {
        bells: NOTES.iter().map(|&n| bell(fs, n)).collect(),
        rng: Rng::new(0x57ea_4011),
        space: Space::outdoors(fs),
        room: 1.0,
        drive: 0.0,
        wander_coef: one_pole(fs, 9.0),
        hiss_hp: one_pole(fs, 1200.0),
        hiss_lp: one_pole(fs, 6000.0),
        hp_state: 0.0,
        lp_state: 0.0,
    })
}

impl Instrument for SteamWhistle {
    fn control(&mut self, p: &[f64], gate: f64, _dt: f64) {
        self.drive = p[STEAM] * gate;
        self.room = p[ROOM];
        for (i, b) in self.bells.iter_mut().enumerate() {
            b.on = p[CHIMES] > i as f64 + 0.5;
            // a weak jet speaks flat; full steam is about a third of a semitone sharp
            b.jet.f = b.note * (0.94 + 0.08 * self.drive);
            b.jet.coupling = 0.8;
            b.net.begin_block(BLOCK);
        }
    }
    #[inline]
    fn sample(&mut self) -> f64 {
        let drive = self.drive;
        // the hiss: white noise in a band (a low-passed copy taken off, then smoothed)
        let w = self.rng.bipolar();
        self.hp_state += self.hiss_hp * (w - self.hp_state);
        self.lp_state += self.hiss_lp * ((w - self.hp_state) - self.lp_state);
        let hiss = self.lp_state * drive * sqrt(drive);
        let mut y = 0.0;
        for b in self.bells.iter_mut() {
            if !b.on {
                continue;
            }
            b.wander += self.wander_coef * (self.rng.bipolar() * 6.0 - b.wander);
            let d = drive * (1.0 + 0.25 * b.wander).max(0.0);
            let p_local = b.net.p[b.pipe.first_node()];
            // the oscillator swings about ±2; the jet's flow flattens as it swings fully in and out
            let out = b.jet.next(d, p_local, 1.0);
            let x = if d > 0.02 { out / d } else { 0.0 };
            // the jet swings in and out of the bell's mouth: a volume flow just inside it
            b.net.ext_flow[b.pipe.node(1)] += JET_FLOW * d * tanh(0.9 * x) + HISS_IN * hiss;
            y += b.net.step();
        }
        self.space.next_mix(y + HISS_OUT * hiss, self.room)
    }
}

pub fn spec() -> Spec {
    let k = Key::at;
    Spec {
        id: "steamwhistle",
        name: "Steam whistle",
        family: Family::Aerophone,
        blurb: "Sheets of steam blown across the mouths of three stopped bells; each jet locks onto its bell's resonance.",
        params: vec![
            Param::new("steam", "Steam", Kind::Action, 0.05, 1.0, 0.8).smooth(0.08),
            Param::new("chimes", "Bells sounding", Kind::Action, 1.0, 3.0, 3.0),
            Param::new("room", "Open country", Kind::Space, 0.0, 1.0, 1.0).smooth(0.05),
        ],
        phrases: vec![
            Phrase {
                id: "crossing",
                name: "Grade crossing",
                keys: vec![
                    k(0.0, &[("gate", 0.0), ("steam", 0.85)]),
                    k(0.05, &[("gate", 1.0)]),
                    k(1.8, &[("gate", 1.0)]),
                    k(1.9, &[("gate", 0.0)]),
                    k(2.6, &[("gate", 0.0)]),
                    k(2.65, &[("gate", 1.0)]),
                    k(4.4, &[("gate", 1.0)]),
                    k(4.5, &[("gate", 0.0)]),
                    k(5.2, &[("gate", 0.0)]),
                    k(5.25, &[("gate", 1.0)]),
                    k(5.7, &[("gate", 1.0)]),
                    k(5.8, &[("gate", 0.0)]),
                    k(6.5, &[("gate", 0.0)]),
                    k(6.55, &[("gate", 1.0)]),
                    k(9.5, &[("gate", 1.0), ("steam", 0.85)]),
                    k(11.0, &[("steam", 0.3)]),
                    k(11.3, &[("gate", 0.0)]),
                ],
            },
            Phrase { id: "wail", name: "Wail", keys: vec![k(0.0, &[("gate", 0.0), ("steam", 0.1)]), k(0.05, &[("gate", 1.0)]), k(1.8, &[("steam", 1.0)]), k(3.0, &[("steam", 1.0)]), k(5.0, &[("steam", 0.15)]), k(5.3, &[("gate", 0.0)])] },
        ],
        gate: (0.25, 0.35),
        gain: 2.8e-8,
        reference: None,
        build,
    }
}
