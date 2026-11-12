//! Dog: a mammal's larynx under a throat, a long mouth and a nose.
//!
//! lungs ─ windpipe ─ folds ─ throat ─┬─ mouth → air
//!                                    └─ nose → air
//!
//! The same parts as a human voice, in other proportions. What makes the calls a dog's is what
//! the dog does with them:
//!  - A bark is one short push of breath while the jaw snaps open and half shut and the pitch
//!    jumps up and falls back.
//!  - A growl is slow, loose, irregular fold vibration behind a nearly closed mouth.
//!  - A howl is a long, clean note that climbs and holds.
//!  - A whine is high and thin, with the mouth shut, so it comes out through the nose.
//!
//! The pitches and lengths of the phrases are those measured from recordings of real dogs
//! (`references/SOURCES.md`). The airway's proportions are a medium dog's; `size` makes it a
//! smaller or larger one. CHIMERA's mammal airway and fold model are used as they are.

use crate::air::Decimator;
use crate::air::mammal::{MammalAirway, MammalSpec, mammal_airway, mammal_noise, step_glottis};
use crate::air::{CMH2O, fill_areas};
use crate::instrument::{BLOCK, Curve, Family, Instrument, Key, Kind, Param, Phrase, Reference, Spec};
use crate::parts::space::Space;

const PITCH: usize = 0;
const PRESSURE: usize = 1;
const MOUTH: usize = 2;
const ROUGH: usize = 3;
const BREATHY: usize = 4;
const SIZE: usize = 5;
const ROOM: usize = 6;

/// A medium dog's airway: (length cm, area cm²). Throat and mouth together are 16 cm.
const WINDPIPE: (f64, f64) = (18.0, 2.5);
const THROAT: (f64, f64) = (6.0, 2.6);
const MOUTH_TUBE: (f64, f64) = (10.0, 3.2);
const NOSE: (f64, f64) = (9.0, 1.2);
/// The smallest and largest dog, as a multiple of the medium one's lengths.
const SMALLEST: f64 = 0.7;
const LARGEST: f64 = 1.5;
/// The simulation runs this many times faster than the sound is heard: the tubes are short.
const OVER: usize = 2;

struct Dog {
    a: MammalAirway,
    down: Decimator,
    space: Space,
    room: f64,
    /// the size the airway was last shaped for
    size: f64,
    mouth: f64,
}

fn build(fs: f64) -> Box<dyn Instrument> {
    // sections sized so the airway can shrink to the smallest dog and stay stable
    let a = mammal_airway(
        fs * OVER as f64,
        MammalSpec { trachea: WINDPIPE, pharynx: THROAT, oral: MOUTH_TUBE, nasal: Some(NOSE), wall: None, wall_radiation: Some(0.3), coarse: Some(1.0 / SMALLEST + 0.05), nasal_radiates: None },
        |_, _| {},
    );
    Box::new(Dog { a, down: Decimator::new(OVER), space: Space::hall(fs), room: 0.0, size: f64::NAN, mouth: f64::NAN })
}

impl Instrument for Dog {
    fn control(&mut self, p: &[f64], gate: f64, _dt: f64) {
        let a = &mut self.a;
        a.net.set_fixed_pressure(a.lungs, p[PRESSURE] * CMH2O * gate);
        let (size, mouth) = (p[SIZE], p[MOUTH]);
        let area = size * size;
        // the folds: looser and more irregular as the voice roughens, leakier as it turns breathy
        let g = &mut a.glottis_src.params;
        g.f0 = p[PITCH];
        g.amplitude = 0.2 * area;
        // a dog's folds close softly: measured barks and growls have little above 1.5 kHz
        g.open_quotient = 0.78 + 0.08 * p[BREATHY];
        g.closure_sharpness = 0.0;
        g.effort = 0.35;
        g.chink = (0.01 + 0.08 * p[BREATHY]) * area;
        g.abduction = 0.12 * p[BREATHY] * area;
        g.chaos = p[ROUGH];
        g.jitter = 0.008 + 0.05 * p[ROUGH];
        g.shimmer = 0.03 + 0.12 * p[ROUGH];
        // the airway: its length follows the dog's size; the front of the mouth follows the jaw
        if self.size != size || self.mouth != mouth {
            (self.size, self.mouth) = (size, mouth);
            let throat = fill_areas(a.pharynx, |_| THROAT.1 * area, &mut a.phar_area);
            a.net.set_tube_targets(a.pharynx, throat, Some(THROAT.0 * size), None);
            let front = (0.15 + 4.6 * mouth) * area;
            let oral = fill_areas(a.oral, |u| if u > 0.6 { MOUTH_TUBE.1 * area + (front - MOUTH_TUBE.1 * area) * (u - 0.6) / 0.4 } else { MOUTH_TUBE.1 * area }, &mut a.oral_area);
            a.net.set_tube_targets(a.oral, oral, Some(MOUTH_TUBE.0 * size), None);
            if let Some(nose) = a.nasal {
                let n = [NOSE.1 * area; 64];
                a.net.set_tube_targets(nose, &n[..nose.n], Some(NOSE.0 * size), None);
            }
            // a shut mouth sends the sound through the nose
            if let Some(velum) = a.velum {
                a.net.set_port_target(velum, (0.05 + 0.9 * (1.0 - mouth)) * area);
            }
        }
        mammal_noise(a, 2.5e-6 * (0.1 + p[BREATHY] + 0.1 * p[ROUGH]), Some(2e-7));
        self.room = p[ROOM];
        a.net.begin_block(BLOCK * OVER);
    }
    fn settle(&mut self) {
        self.a.net.snap_to_targets();
    }
    #[inline]
    fn sample(&mut self) -> f64 {
        loop {
            step_glottis(&mut self.a);
            let y = self.a.net.step();
            if self.down.push(y) {
                return self.space.next_mix(self.down.value, self.room);
            }
        }
    }
}

/// One bark starting at `t`: the breath pushes for an eighth of a second, the jaw snaps open and
/// half shut, and the pitch jumps up and falls back (measured barks: 0.16–0.32 s, 340–680 Hz).
fn bark(t: f64, keys: &mut Vec<Key>) {
    keys.push(Key::at(t, &[("gate", 1.0), ("pitch", 430.0), ("mouth", 0.25), ("pressure", 24.0), ("rough", 0.35), ("breathy", 0.1)]).curve(Curve::Step));
    keys.push(Key::at(t + 0.04, &[("pitch", 660.0), ("mouth", 1.0)]).curve(Curve::Ratio));
    keys.push(Key::at(t + 0.12, &[("pitch", 370.0), ("mouth", 0.4), ("gate", 1.0)]).curve(Curve::Ratio));
    keys.push(Key::at(t + 0.125, &[("gate", 0.0)]).curve(Curve::Step));
}

fn barks(times: &[f64]) -> Vec<Key> {
    let mut keys = Vec::new();
    for &t in times {
        bark(t, &mut keys);
    }
    keys
}

pub fn spec() -> Spec {
    let k = Key::at;
    Spec {
        id: "dog",
        name: "Dog",
        family: Family::Aerophone,
        blurb: "A mammal's larynx under a throat, a long mouth and a nose; the jaw, the breath and the folds make the bark, growl, howl and whine.",
        params: vec![
            Param::new("pitch", "Pitch", Kind::Action, 60.0, 1800.0, 450.0).unit("Hz").log().smooth(0.008),
            Param::new("pressure", "Breath pressure", Kind::Action, 2.0, 40.0, 14.0).unit("cmH₂O").smooth(0.01),
            Param::new("mouth", "Jaw open", Kind::Action, 0.0, 1.0, 0.5).smooth(0.012),
            Param::new("rough", "Rough", Kind::Action, 0.0, 1.0, 0.2).smooth(0.03),
            Param::new("breathy", "Breathy", Kind::Action, 0.0, 1.0, 0.1).smooth(0.03),
            Param::new("size", "Size (1 is a medium dog)", Kind::Design, SMALLEST, LARGEST, 1.0),
            Param::new("room", "Hall", Kind::Space, 0.0, 1.0, 0.0).smooth(0.05),
        ],
        phrases: vec![
            Phrase { id: "bark", name: "Bark", keys: barks(&[0.0]) },
            Phrase { id: "barks", name: "Three barks", keys: barks(&[0.0, 0.7, 1.25]) },
            // measured growls: two seconds and more, 100–210 Hz, rough for most of their length
            Phrase {
                id: "growl",
                name: "Growl",
                keys: vec![
                    k(0.0, &[("gate", 1.0), ("pitch", 150.0), ("mouth", 0.25), ("pressure", 16.0), ("rough", 0.9), ("breathy", 0.08)]).curve(Curve::Step),
                    k(0.9, &[("pitch", 115.0), ("pressure", 19.0)]).curve(Curve::Ratio),
                    k(1.7, &[("pitch", 140.0), ("mouth", 0.35)]).curve(Curve::Ratio),
                    k(2.3, &[("pitch", 105.0), ("gate", 1.0), ("pressure", 12.0)]).curve(Curve::Ratio),
                    k(2.4, &[("gate", 0.0)]).curve(Curve::Step),
                ],
            },
            // the measured howl is a small dog's, 500–1600 Hz; this is a medium dog's, lower
            Phrase {
                id: "howl",
                name: "Howl",
                keys: vec![
                    k(0.0, &[("gate", 1.0), ("pitch", 480.0), ("mouth", 0.45), ("pressure", 16.0), ("rough", 0.03), ("breathy", 0.1)]).curve(Curve::Step),
                    k(0.5, &[("pitch", 880.0)]).curve(Curve::Ratio),
                    k(2.0, &[("pitch", 820.0)]).curve(Curve::Ratio),
                    k(2.6, &[("pitch", 520.0), ("gate", 1.0), ("pressure", 9.0)]).curve(Curve::Ratio),
                    k(2.7, &[("gate", 0.0)]).curve(Curve::Step),
                ],
            },
            // no recording of a dog's whine was found to measure; these numbers are a guess
            Phrase {
                id: "whine",
                name: "Whine",
                keys: vec![
                    k(0.0, &[("gate", 1.0), ("pitch", 950.0), ("mouth", 0.03), ("pressure", 7.0), ("rough", 0.0), ("breathy", 0.25)]).curve(Curve::Step),
                    k(0.35, &[("pitch", 1350.0)]).curve(Curve::Ratio),
                    k(0.9, &[("pitch", 900.0), ("gate", 1.0)]).curve(Curve::Ratio),
                    k(1.0, &[("gate", 0.0)]).curve(Curve::Step),
                ],
            },
        ],
        gate: (0.008, 0.03),
        gain: 1.0e-7,
        reference: Some(Reference {
            title: "Barking of a dog",
            url: "https://commons.wikimedia.org/wiki/File:Barking_of_a_dog.ogg",
            license: "CC BY-SA 3.0",
            fit: "Bark: 0.18 s at 360–590 Hz (real: 0.16–0.32 s, 340–680 Hz). Growl: 2.4 s near 120 Hz, clear 64% of the time, band energies within 5 dB of one real growl up to 700 Hz and above 1.5 kHz (real: 130–210 Hz, clear 22–52%). Howl: band energies within 4 dB of a real one. Not right yet: the bark has about 10 dB too much between 1.5 and 3 kHz; the airway is a guess at a medium dog, not fitted; the whine was not measured against anything.",
        }),
        credit: "Assembled in Bayin from CHIMERA's mammal airway and fold model.",
        build,
    }
}
