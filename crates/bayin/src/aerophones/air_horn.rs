//! Locomotive air horn: five chimes, each a metal diaphragm buzzing into its own flaring horn.
//! The diaphragm is a valve the air pushes open, like a brass player's lips but stiffer; the
//! horn's length sets the note, as the bore does for brass. The five notes are the chord of a
//! common five-chime horn (D♯4, F♯4, G♯4, B4, D♯5). Fewer chimes sounding gives the simpler horns.
//!
//! Each chime's design was found by search: on its note within 5 cents, sounding steadily, with
//! harmonics near a sawtooth's. In every one the diaphragm rests pressed against its seat and the
//! air lifts it off, so it shuts once a cycle and chops the air into pulses, which is where a
//! horn's edge comes from. It has not yet been fitted to a recording of a real horn.

use crate::air::math::{PI, Rng, exp, ln};
use crate::air::{AirwayNetwork, CMH2O, C_SOUND, LipReed, PortSpec, Tube, TubeArea, TubeOpts, lungs, tube};
use crate::instrument::{BLOCK, Family, Instrument, Key, Kind, Param, Phrase, Spec};
use crate::parts::space::Space;
use crate::parts::steepen::Steepen;

const PRESSURE: usize = 0;
const CHIMES: usize = 1;
const TUNE: usize = 2;
const ROOM: usize = 3;

/// The chimes' notes (Hz).
const NOTES: [f64; 5] = [311.13, 369.99, 415.30, 493.88, 622.25];
/// A horn's mouth area (cm²).
const MOUTH: f64 = 110.0;
/// Throat area (cm²).
const THROAT: [f64; 5] = [1.2343, 0.8044, 1.0825, 1.3002, 1.3002];
/// Horn length as a fraction of the half wavelength of its note.
const LENGTH_FACTOR: [f64; 5] = [1.0276, 1.1693, 1.0089, 0.9824, 0.9824];
/// The diaphragm's own resonance as a fraction of its note.
const REED_FACTOR: [f64; 5] = [0.6587, 0.7192, 0.5864, 0.5759, 0.5759];
/// How far full pressure pushes the diaphragm off its rest (cm).
const PUSH: [f64; 5] = [0.1562, 0.1039, 0.0567, 0.0802, 0.0802];
/// The diaphragm's rest position (cm; negative: pressed onto its seat).
const SEAT: [f64; 5] = [-0.0411, -0.0395, -0.0363, -0.0388, -0.0388];
/// The slit the lifted diaphragm opens (cm wide).
const WIDTH: [f64; 5] = [1.4562, 3.8621, 3.7472, 3.8594, 3.8594];
const DAMPING: [f64; 5] = [0.1003, 0.1200, 0.0449, 0.0420, 0.0420];

struct Chime {
    net: AirwayNetwork,
    chamber: Tube,
    horn: Tube,
    supply: usize,
    valve: usize,
    reed: LipReed,
    note: f64,
    /// 0..1: how far this chime's air valve is open
    on: f64,
    steepen: Steepen,
    /// the note's period (samples) and how far its crests run ahead, as a fraction of it
    period: f64,
    lean: f64,
}

struct AirHorn {
    chimes: Vec<Chime>,
    space: Space,
    room: f64,
    rng: Rng,
    /// the air supply's slow unsteadiness (−1..1) and how fast it moves
    wander: f64,
    wander_coef: f64,
}

fn chime(fs: f64, i: usize) -> Chime {
    let (note, throat) = (NOTES[i], THROAT[i]);
    let mut net = AirwayNetwork::new(fs);
    let length = LENGTH_FACTOR[i] * C_SOUND / (2.0 * note);
    let m = ln(MOUTH / throat);
    let chamber = tube(&mut net, "chamber", fs, 3.0, TubeArea::Uniform(12.0), TubeOpts::default());
    let horn = tube(&mut net, "horn", fs, length, TubeArea::Profile(&|u| throat * exp(m * u)), TubeOpts { max: Some(160.0), ..Default::default() });
    let supply = lungs(&mut net, chamber.first_node(), 12.0);
    let valve = net.add_port(chamber.last_node(), horn.first_node(), PortSpec::new(0.0, 0.3).kinetic(0.3, 1.0));
    net.add_radiation(horn.last_node());
    net.finalize();
    Chime { net, chamber, horn, supply, valve, reed: LipReed::new(fs), note, on: 0.0, steepen: Steepen::new(fs, 0.3 * fs / note), period: fs / note, lean: 0.0 }
}

fn build(fs: f64) -> Box<dyn Instrument> {
    let chimes = (0..NOTES.len()).map(|i| chime(fs, i)).collect();
    Box::new(AirHorn { chimes, space: Space::outdoors(fs), room: 1.0, rng: Rng::new(0xa117_0421), wander: 0.0, wander_coef: 1.0 - exp(-2.0 * PI * 6.0 / fs) })
}

impl Instrument for AirHorn {
    fn control(&mut self, p: &[f64], gate: f64, _dt: f64) {
        self.room = p[ROOM];
        for (i, c) in self.chimes.iter_mut().enumerate() {
            // chime i sounds once the count passes it; a fractional count half-opens the next one
            c.on = (p[CHIMES] - i as f64).clamp(0.0, 1.0);
            // the supply is never quite steady: a few percent, slowly
            c.net.set_fixed_pressure(c.supply, p[PRESSURE] * CMH2O * gate * c.on * (1.0 + 0.04 * self.wander));
            c.reed.f = c.note * REED_FACTOR[i] * p[TUNE];
            // a horn blown harder leans its wave further toward a sawtooth
            c.lean = (0.16 * p[PRESSURE] / 70.0).min(0.26) * gate;
            c.reed.zeta = DAMPING[i];
            c.reed.h0 = SEAT[i];
            c.reed.width = WIDTH[i];
            // the air pushes every chime's diaphragm the same distance off its rest at full
            // pressure: a stiffer, higher diaphragm needs more surface per mass to move as far.
            // Pushed much further than its seat, the air holds it wide open instead of letting
            // it buzz.
            let w = 2.0 * PI * c.reed.f;
            c.reed.surface_per_mass = PUSH[i] * w * w / (70.0 * CMH2O);
            c.net.begin_block(BLOCK);
        }
    }
    fn settle(&mut self) {
        for c in self.chimes.iter_mut() {
            c.net.snap_to_targets();
        }
    }
    #[inline]
    fn sample(&mut self) -> f64 {
        self.wander += self.wander_coef * (self.rng.bipolar() * 6.0 - self.wander);
        let mut y = 0.0;
        for c in self.chimes.iter_mut() {
            if c.on <= 0.0 {
                continue;
            }
            let p = &c.net.p;
            let dp = p[c.chamber.last_node()] - p[c.horn.first_node()];
            let open = c.reed.next(dp);
            c.net.set_port_area(c.valve, open);
            let out = c.net.step();
            y += c.steepen.next(out, c.lean * c.period);
        }
        self.space.next_mix(y, self.room)
    }
}

/// The gate through a signal of blasts: (seconds on, seconds off) each.
fn blasts(pattern: &[(f64, f64)]) -> Vec<Key> {
    let mut keys = vec![Key::at(0.0, &[("gate", 0.0)])];
    let mut t = 0.02;
    for &(on, off) in pattern {
        keys.push(Key::at(t, &[("gate", 1.0)]));
        keys.push(Key::at(t + on, &[("gate", 1.0)]));
        keys.push(Key::at(t + on + 0.05, &[("gate", 0.0)]));
        t += on + 0.05 + off;
        keys.push(Key::at(t - 0.02, &[("gate", 0.0)]));
    }
    keys
}

pub fn spec() -> Spec {
    Spec {
        id: "airhorn",
        name: "Locomotive air horn",
        family: Family::Aerophone,
        blurb: "Five diaphragms, each buzzing into its own flaring horn, sounding a chord.",
        params: vec![
            Param::new("pressure", "Air pressure", Kind::Action, 10.0, 120.0, 70.0).unit("cmH₂O"),
            Param::new("chimes", "Chimes sounding", Kind::Action, 1.0, 5.0, 5.0).smooth(0.02),
            Param::new("tune", "Diaphragm tuning", Kind::Design, 0.9, 1.1, 1.0).smooth(0.05),
            Param::new("room", "Open country", Kind::Space, 0.0, 1.0, 1.0).smooth(0.05),
        ],
        phrases: vec![
            // the grade-crossing signal: two long, one short, one long
            Phrase { id: "crossing", name: "Grade crossing", keys: blasts(&[(1.58, 0.57), (1.58, 0.57), (0.48, 0.57), (2.48, 0.1)]) },
            Phrase { id: "toot", name: "Toot", keys: blasts(&[(0.48, 0.1)]) },
        ],
        gate: (0.03, 0.12),
        gain: 1.6e-9,
        reference: None,
        credit: "Assembled in Bayin from CHIMERA's tubes and lip valve.",
        build,
    }
}
