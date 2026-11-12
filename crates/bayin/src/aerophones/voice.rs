//! The human voice: lungs, vocal folds, and a throat, mouth and nose whose shape the tongue,
//! jaw, lips and soft palate set.
//!
//! lungs ─ folds ─ pharynx ─┬─ mouth ─ lips → air
//!                          └─ soft palate ─ nose → air
//!
//! The whole model is CHIMERA's voice, used as it is. What Bayin adds is the sheet: every
//! articulator is an action with the range the engine gives it, the body (how far toward a
//! woman's or a child's) is the design, and the performance is written down next to the sound.
//!
//! The articulators have their own inertia: an action is where one is asked to go, and it gets
//! there in tens of milliseconds, as in a mouth.

use crate::air::voice::{ARTIC_SPECS, ArticKey, ArticPatch, BodyMix, HumanVoice, N_ARTIC, NEUTRAL, SourcePatch, oversample_for, vowel_artic};
use crate::instrument::{BLOCK, Curve, Family, Instrument, Key, Kind, Param, Phrase, Spec};
use crate::parts::space::Space;

const PITCH: usize = 0;
const BREATH: usize = 1;
const BREATHY: usize = 2;
const PRESSED: usize = 3;
const TIP: usize = 4;
/// the first articulator; the others follow in the engine's order
const ARTIC: usize = 5;
const FEMALE: usize = ARTIC + N_ARTIC;
const CHILD: usize = FEMALE + 1;
const ROOM: usize = CHILD + 1;

/// What each articulator is, in the engine's order.
const LABELS: [(&str, &str); N_ARTIC] = [
    ("Jaw opening", "cm"),
    ("Tongue body, back to front", "cm"),
    ("Tongue body, low to high", "cm"),
    ("Tongue tip, back to front", "cm"),
    ("Tongue tip, low to high", "cm"),
    ("Tongue tip curled back", ""),
    ("Tongue root, back to forward", ""),
    ("Lips apart", "cm"),
    ("Lips pushed forward", "cm"),
    ("Lips spread", ""),
    ("Lower lip to upper teeth", ""),
    ("Soft palate lowered", ""),
    ("Larynx, low to high", "cm"),
    ("Twang (narrowed above the folds)", ""),
    ("False folds closed in", ""),
    ("Tongue sides lowered", ""),
    ("Tongue back raised", ""),
];

struct Voice {
    fs: f64,
    voice: HumanVoice,
    /// the body the voice was built for
    body: (f64, f64),
    /// the block of sound made at the last `control`, and how far into it `sample` is
    block: [f32; BLOCK],
    at: usize,
    space: Space,
    room: f64,
}

fn build(fs: f64) -> Box<dyn Instrument> {
    Box::new(Voice { fs, voice: HumanVoice::new(fs, BodyMix::MALE.body(), oversample_for(fs)), body: (0.0, 0.0), block: [0.0; BLOCK], at: BLOCK, space: Space::hall(fs), room: 0.0 })
}

impl Instrument for Voice {
    fn control(&mut self, p: &[f64], gate: f64, _dt: f64) {
        // another body is another voice: it starts from rest
        if self.body != (p[FEMALE], p[CHILD]) {
            self.body = (p[FEMALE], p[CHILD]);
            self.voice = HumanVoice::new(self.fs, BodyMix::new(p[FEMALE], p[CHILD]).body(), oversample_for(self.fs));
        }
        self.voice.set_source(&SourcePatch { f0: Some(p[PITCH]), pressure: Some(p[BREATH]), breathiness: Some(p[BREATHY]), tension: Some(p[PRESSED]), ..Default::default() });
        let mut posture = ArticPatch::default();
        for (i, k) in ArticKey::ALL.iter().enumerate() {
            posture.set(*k, p[ARTIC + i]);
        }
        // the tip either goes where it is asked or rests behind the lower teeth
        self.voice.set_articulation(&posture, Some(p[TIP] < 0.5));
        self.voice.set_gate(gate);
        self.room = p[ROOM];
        self.voice.render(&mut self.block);
        self.at = 0;
    }
    #[inline]
    fn sample(&mut self) -> f64 {
        let y = self.block[self.at.min(BLOCK - 1)] as f64;
        self.at += 1;
        self.space.next_mix(y, self.room)
    }
}

/// A posture for a vowel, from the engine's calibrated table (for a man's body), as phrase values.
fn vowel(ipa: &str) -> Vec<(&'static str, f64)> {
    let patch = vowel_artic(ipa, BodyMix::MALE).unwrap_or_else(|| panic!("no vowel {ipa}"));
    ArticKey::ALL.iter().filter_map(|k| patch.get(*k).map(|x| (k.name(), x))).collect()
}

/// Five vowels on one breath, gliding from each to the next.
fn vowels() -> Vec<Key> {
    let mut keys = Vec::new();
    for (i, ipa) in ["a", "e", "i", "o", "u"].iter().enumerate() {
        let t = i as f64 * 0.7;
        let mut set = vowel(ipa);
        set.push(("gate", 1.0));
        // hold each vowel, then move to the next over the last 150 ms
        keys.push(Key::at(t, &set).curve(if i == 0 { Curve::Step } else { Curve::Line }));
        keys.push(Key::at(t + 0.55, &set));
    }
    keys.push(Key::at(3.4, &[("gate", 0.0)]).curve(Curve::Step));
    keys
}

/// One vowel sliding up an octave and back.
fn siren() -> Vec<Key> {
    let mut open = vowel("a");
    open.extend([("gate", 1.0), ("pitch", 110.0)]);
    vec![Key::at(0.0, &open).curve(Curve::Step), Key::at(1.4, &[("pitch", 220.0)]).curve(Curve::Ratio), Key::at(2.8, &[("pitch", 110.0)]).curve(Curve::Ratio), Key::at(3.0, &[("gate", 0.0)]).curve(Curve::Step)]
}

pub fn spec() -> Spec {
    use Kind::{Action, Design};
    // the voice has its own inertia and its own breath, so the player passes actions straight on
    const NOW: f64 = 0.001;
    let mut params = vec![
        Param::new("pitch", "Pitch", Action, 50.0, 900.0, 110.0).unit("Hz").log().smooth(NOW),
        Param::new("breath", "Lung pressure", Action, 2.0, 22.0, 8.0).unit("cmH₂O").smooth(NOW),
        Param::new("breathy", "Breathy", Action, 0.0, 1.0, 0.15).smooth(NOW),
        Param::new("pressed", "Pressed", Action, 0.0, 1.0, 0.5).smooth(NOW),
        Param::new("tip", "Tongue tip goes where asked (1) or rests (0)", Action, 0.0, 1.0, 0.0).smooth(NOW),
    ];
    for (i, k) in ArticKey::ALL.iter().enumerate() {
        let s = ARTIC_SPECS[i];
        params.push(Param::new(k.name(), LABELS[i].0, Action, s.min, s.max, NEUTRAL.get(*k)).unit(LABELS[i].1).smooth(NOW));
    }
    params.extend([
        Param::new("female", "Toward a woman's body", Design, 0.0, 1.0, 0.0).smooth(NOW),
        Param::new("child", "Toward a child's body", Design, 0.0, 1.0, 0.0).smooth(NOW),
        Param::new("room", "Hall", Kind::Space, 0.0, 1.0, 0.0).smooth(0.05),
    ]);
    Spec {
        id: "voice",
        name: "Human voice",
        family: Family::Aerophone,
        blurb: "Vocal folds under a throat, mouth and nose shaped by the tongue, jaw, lips and soft palate.",
        params,
        phrases: vec![Phrase { id: "vowels", name: "Five vowels", keys: vowels() }, Phrase { id: "siren", name: "Siren", keys: siren() }],
        gate: (NOW, NOW),
        gain: 1.0,
        reference: None,
        credit: "CHIMERA's voice model, by Jameson Hodge, unchanged.",
        build,
    }
}
