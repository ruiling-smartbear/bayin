//! CHIMERA's creatures, as they are: a songbird's two-voiced syrinx, a seal's sweeping trills, an
//! alligator's bellow and an elephant's rumble and trumpet. Their physics, parameters and calls
//! are CHIMERA's; this file only reads each one's definition into a Bayin sheet and steps its
//! simulation.
//!
//! Some of them simulate faster than the sound is heard (the songbird four times over, for its
//! kilohertz notes); their sound is brought down to the output rate with CHIMERA's own filter.

use crate::air::bestiary::{CreatureDef, CreatureSim, Curve as TheirCurve, Decimator, creature_by_id};
use crate::instrument::{BLOCK, Curve, Family, Instrument, Key, Kind, Param, Phrase, Spec};

struct Creature {
    sim: Box<dyn CreatureSim>,
    /// simulated samples per sample of sound
    over: usize,
    down: Decimator,
}

fn build(id: &str, fs: f64) -> Box<dyn Instrument> {
    let def = creature_by_id(id).unwrap_or_else(|| panic!("CHIMERA has no creature {id}"));
    let over = def.oversample.max(1) as usize;
    Box::new(Creature { sim: def.create(fs * over as f64), over, down: Decimator::new(over) })
}

impl Instrument for Creature {
    fn control(&mut self, p: &[f64], gate: f64, dt: f64) {
        self.sim.control(p, gate, dt);
        if let Some(net) = self.sim.net_mut() {
            net.begin_block(BLOCK * self.over);
        }
    }
    fn settle(&mut self) {
        if let Some(net) = self.sim.net_mut() {
            net.snap_to_targets();
        }
    }
    #[inline]
    fn sample(&mut self) -> f64 {
        loop {
            let y = self.sim.sample();
            if self.down.push(y) {
                return self.down.value;
            }
        }
    }
}

/// A creature's definition as a sheet. Its name, description, parameters and calls are CHIMERA's;
/// the calls become phrases, and every parameter is an action.
fn sheet(id: &'static str, gain: f64, build: fn(f64) -> Box<dyn Instrument>) -> Spec {
    let def: &'static CreatureDef = creature_by_id(id).unwrap_or_else(|| panic!("CHIMERA has no creature {id}"));
    let params = def
        .params
        .iter()
        .map(|p| {
            let mut q = Param::new(p.key, p.label, Kind::Action, p.min, p.max, p.default).unit(p.unit.unwrap_or("")).smooth(p.smooth.unwrap_or(0.03));
            q.log = p.log == Some(true);
            q
        })
        .collect();
    let phrases = def
        .calls
        .iter()
        .map(|c| Phrase {
            id: c.id,
            name: c.name,
            keys: c
                .keys
                .iter()
                .map(|k| Key {
                    t: k.t,
                    set: k.v.0.clone(),
                    curve: match k.curve {
                        Some(TheirCurve::Exp) => Curve::Ratio,
                        Some(TheirCurve::Step) => Curve::Step,
                        _ => Curve::Line,
                    },
                })
                .collect(),
        })
        .collect();
    let gate = def.gate_times.map_or((0.012, 0.06), |g| (g.attack, g.release));
    Spec { id, name: def.name, family: Family::Aerophone, blurb: def.blurb, params, phrases, gate, gain, reference: None, credit: "CHIMERA's creature, by Jameson Hodge, unchanged.", build }
}

// Each creature's gain is the one its CHIMERA simulation reports.

pub fn songbird() -> Spec {
    sheet("songbird", 4.8e-7, |fs| build("songbird", fs))
}

pub fn seal() -> Spec {
    sheet("seal", 6.4e-8, |fs| build("seal", fs))
}

pub fn alligator() -> Spec {
    sheet("alligator", 2.8e-7, |fs| build("alligator", fs))
}

pub fn elephant() -> Spec {
    sheet("elephant", 1.8e-6, |fs| build("elephant", fs))
}
