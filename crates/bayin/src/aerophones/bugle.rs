//! Bugle: a player's lips buzzing into a mouthpiece, a narrow brass bore and a flaring bell.
//!
//! mouth ─ lips ─ cup ─ throat ─ backbore ─ bore ─ bell → air
//!
//! The lips are a valve that the breath pushes open and their own spring pulls shut. The bore is
//! narrow, rigid and long, so its resonances are strong and the lips lock onto them: the bugle's
//! natural notes.
//!
//! The mouthpiece is what makes it brass and not a pipe: the cup and its narrow throat resonate
//! together around a kilohertz and lift the harmonics there. Blown harder, the wave in the bore
//! leans toward a sawtooth (`parts::steepen`), so the tone brightens as it gets louder.
//!
//! Every dimension is a parameter. The defaults were fitted by search to the notes of a real G
//! bugle (`tools/calibrate_bugle.py`).

use crate::air::math::{PI, cbrt, exp, ln, pow, sqrt};
use crate::air::{AirwayNetwork, CMH2O, LipReed, PortSpec, Tube, TubeArea, TubeOpts, WALL_SOFT, fill_areas, lungs, tube};
use crate::instrument::{BLOCK, Curve, Family, Instrument, Key, Kind, Param, Phrase, Reference, Spec};
use crate::parts::space::Space;
use crate::parts::steepen::Steepen;

// what the player does
const PITCH: usize = 0;
const PRESSURE: usize = 1;
// the instrument
const CUP: usize = 2;
const THROAT: usize = 3;
const BACKBORE: usize = 4;
const BORE: usize = 5;
const FLARE_START: usize = 6;
const FLARE_EXP: usize = 7;
const BELL: usize = 8;
const LENGTH: usize = 9;
// the player's lips
const LIP_BELOW: usize = 10;
const LIP_DAMPING: usize = 11;
const LIP_WIDTH: usize = 12;
const LIP_PUSH: usize = 13;
const LIP_REST: usize = 14;
/// how far the wave leans at 30 cmH₂O (a fraction of the period)
const LEAN: usize = 15;
/// how much of the hall is heard
const ROOM: usize = 16;

/// The fitted design: the parameters from CUP to LEAN.
const DESIGN: [f64; 14] = [2.3655, 0.1729, 10.7830, 1.1767, 0.5517, 2.3506, 85.1507, 137.9215, 0.7135, 0.1150, 1.2145, 0.0350, -0.0166, 0.2109];
/// The breath the design was fitted at (cmH₂O).
const BREATH: f64 = 62.7;

/// The bore's length when it is built (cm). Its sections are sized for this, so the length
/// parameter can shorten it by a tenth and no more.
const BUILT_LENGTH: f64 = 148.0;

/// Area along the bore (cm²) at u ∈ (0,1): the backbore widens from the throat to the bore, the
/// bore runs straight, and the bell opens faster and faster to its mouth.
fn bore_area(u: f64, d: &[f64]) -> f64 {
    let (throat, backbore, bore, start, e, bell, length) = (d[THROAT], d[BACKBORE], d[BORE], d[FLARE_START], d[FLARE_EXP], d[BELL], d[LENGTH]);
    let x = u * length;
    if x < backbore {
        // a cone: the radius grows evenly
        let (r0, r1) = (sqrt(throat), sqrt(bore));
        let r = r0 + (r1 - r0) * x / backbore;
        return r * r;
    }
    if u < start {
        return bore;
    }
    bore * exp(ln(bell / bore) * pow((u - start) / (1.0 - start), e))
}

struct Bugle {
    net: AirwayNetwork,
    mouth: Tube,
    bore: Tube,
    lungs: usize,
    lip_port: usize,
    cup: usize,
    throat_port: usize,
    lips: LipReed,
    space: Space,
    steepen: Steepen,
    fs: f64,
    /// how far the wave's crests run ahead (samples)
    lean: f64,
    room: f64,
    areas: Vec<f64>,
    /// the design the bore was last shaped for (CUP up to LIP_BELOW)
    shaped: [f64; 8],
}

fn build(fs: f64) -> Box<dyn Instrument> {
    let mut d = [0.0; 17];
    d[CUP..ROOM].copy_from_slice(&DESIGN);
    let mut net = AirwayNetwork::new(fs);
    let mouth = tube(&mut net, "mouth", fs, 6.0, TubeArea::Uniform(9.0), TubeOpts { wall: Some(WALL_SOFT), ..Default::default() });
    let cup = net.add_cavity("cup", d[CUP], None);
    let bore = tube(&mut net, "bore", fs, BUILT_LENGTH, TubeArea::Profile(&|u| bore_area(u, &d)), TubeOpts { max: Some(240.0), coarse: Some(1.12), ..Default::default() });
    let lungs = lungs(&mut net, mouth.first_node(), 9.0);
    let lip_port = net.add_port(mouth.last_node(), cup, PortSpec::new(0.0, 0.4).kinetic(0.3, 1.0));
    let throat_port = net.add_port(cup, bore.first_node(), PortSpec::new(d[THROAT], 0.5).kinetic(0.2, 0.6));
    net.add_radiation(bore.last_node());
    net.finalize();
    Box::new(Bugle { net, mouth, bore, lungs, lip_port, cup, throat_port, lips: LipReed::new(fs), space: Space::hall(fs), steepen: Steepen::new(fs, 0.3 * fs / 250.0), fs, lean: 0.0, room: 1.0, areas: vec![0.0; bore.n], shaped: [f64::NAN; 8] })
}

impl Instrument for Bugle {
    fn control(&mut self, p: &[f64], gate: f64, _dt: f64) {
        self.net.set_fixed_pressure(self.lungs, p[PRESSURE] * CMH2O * gate);
        // the instrument (the cup is a one-section cavity as long as it was built)
        if self.shaped[..] != p[CUP..LIP_BELOW] {
            self.shaped.copy_from_slice(&p[CUP..LIP_BELOW]);
            self.net.set_node_area_target(self.cup, p[CUP] / cbrt(DESIGN[0]));
            self.net.set_port_target(self.throat_port, p[THROAT]);
            let length = p[LENGTH].max(0.9 * BUILT_LENGTH);
            let areas = fill_areas(self.bore, |u| bore_area(u, p), &mut self.areas);
            self.net.set_tube_targets(self.bore, areas, Some(length), None);
        }
        // the lips: a valve the breath pushes open sounds well above its own resonance, so they
        // are tuned below the note asked for
        let lips = &mut self.lips;
        lips.f = p[PITCH] * p[LIP_BELOW];
        lips.zeta = p[LIP_DAMPING];
        lips.h0 = p[LIP_REST];
        lips.width = p[LIP_WIDTH];
        // the breath at 30 cmH₂O pushes the lips this far apart, whatever their pitch
        let w = 2.0 * PI * lips.f;
        lips.surface_per_mass = p[LIP_PUSH] * w * w / (30.0 * CMH2O);
        self.room = p[ROOM];
        // blown harder, the wave leans further toward a sawtooth
        let hard = p[PRESSURE] / 30.0;
        self.lean = (p[LEAN] * hard * hard).min(0.28) * gate * self.fs / p[PITCH];
        self.net.begin_block(BLOCK);
    }
    fn settle(&mut self) {
        self.net.snap_to_targets();
    }
    #[inline]
    fn sample(&mut self) -> f64 {
        let p = &self.net.p;
        let dp = p[self.mouth.last_node()] - p[self.cup];
        let open = self.lips.next(dp);
        self.net.set_port_area(self.lip_port, open);
        let y = self.net.step();
        let y = self.steepen.next(y, self.lean);
        self.space.next_mix(y, self.room)
    }
}

/// A fanfare on the 4th, 5th and 6th harmonics (do, mi, sol). Each note is tongued: the tongue
/// stops the breath for a moment before it, so the old note dies and the new one speaks.
fn fanfare() -> Vec<Key> {
    // a G bugle's notes; the lips lock onto the nearest resonance this bore has
    const DO: f64 = 394.0;
    const MI: f64 = 494.0;
    const SOL: f64 = 594.0;
    // (note, beats); a beat is 0.3 s
    let tune = [(DO, 1.0), (MI, 1.0), (SOL, 2.0), (SOL, 0.5), (MI, 0.5), (SOL, 1.0), (DO, 1.0), (MI, 0.5), (SOL, 0.5), (MI, 1.0), (SOL, 4.0)];
    let mut keys = vec![Key::at(0.0, &[("gate", 0.0), ("pitch", DO)])];
    let mut t = 0.03;
    for (note, beats) in tune {
        keys.push(Key::at(t, &[("pitch", note), ("gate", 1.0)]).curve(Curve::Step));
        t += beats * 0.3;
        keys.push(Key::at(t - 0.045, &[("gate", 1.0)]));
        keys.push(Key::at(t - 0.04, &[("gate", 0.0)]).curve(Curve::Step));
    }
    keys.push(Key::at(t + 0.1, &[("gate", 0.0)]));
    keys
}

pub fn spec() -> Spec {
    use Kind::{Action, Design};
    let d = DESIGN;
    Spec {
        id: "bugle",
        name: "Bugle",
        family: Family::Aerophone,
        blurb: "Lips buzzing into a mouthpiece and a narrow brass bore with a flaring bell; the lips lock onto the bore's resonances.",
        params: vec![
            Param::new("pitch", "Pitch", Action, 150.0, 900.0, 394.0).unit("Hz").log().smooth(0.012),
            Param::new("pressure", "Breath pressure", Action, 5.0, 80.0, BREATH).unit("cmH₂O").smooth(0.02),
            Param::new("cup", "Mouthpiece cup", Design, 0.3, 3.0, d[0]).unit("cm³"),
            Param::new("throat", "Mouthpiece throat", Design, 0.05, 0.3, d[1]).unit("cm²"),
            Param::new("backbore", "Backbore length", Design, 3.0, 14.0, d[2]).unit("cm"),
            Param::new("bore", "Bore", Design, 0.6, 1.5, d[3]).unit("cm²"),
            Param::new("flareStart", "Bell starts at", Design, 0.35, 0.85, d[4]),
            Param::new("flareExp", "Bell curve", Design, 1.2, 7.0, d[5]),
            Param::new("bell", "Bell mouth", Design, 30.0, 220.0, d[6]).unit("cm²"),
            Param::new("length", "Tube length", Design, 134.0, 165.0, d[7]).unit("cm"),
            Param::new("lipBelow", "Lip tuning below the note", Design, 0.35, 1.0, d[8]),
            Param::new("lipDamping", "Lip damping", Design, 0.02, 0.4, d[9]),
            Param::new("lipWidth", "Lip opening width", Design, 0.4, 2.6, d[10]).unit("cm"),
            Param::new("lipPush", "Lip push at 30 cmH₂O", Design, 0.02, 0.8, d[11]).unit("cm"),
            Param::new("lipRest", "Lip rest gap", Design, -0.04, 0.02, d[12]).unit("cm"),
            Param::new("lean", "Wave lean at 30 cmH₂O", Design, 0.0, 0.25, d[13]),
            Param::new("room", "Hall", Kind::Space, 0.0, 1.0, 1.0).smooth(0.05),
        ],
        phrases: vec![
            Phrase { id: "sweep", name: "Lip sweep", keys: vec![Key::at(0.0, &[("gate", 0.0), ("pitch", 250.0)]), Key::at(0.05, &[("gate", 1.0)]), Key::at(5.0, &[("pitch", 800.0)]).curve(Curve::Ratio), Key::at(5.1, &[("gate", 0.0)])] },
            Phrase { id: "fanfare", name: "Fanfare", keys: fanfare() },
        ],
        // tonguing: the breath stops and starts in a few milliseconds
        gate: (0.006, 0.012),
        gain: 4.0e-9,
        reference: Some(Reference {
            title: "Assembly (bugle call), U.S. Marine Corps ceremonial bugler",
            url: "https://commons.wikimedia.org/wiki/File:Assembly_bugle_call.ogg",
            license: "Public domain",
            fit: "Notes at 394, 493 and 594 Hz: within 31 cents, clean, speaking within 0.1 s. Harmonics 1–10 within about 5 dB on average. Not right yet: the 297 Hz note does not sound, and a pitch asked for between two notes gives a rough tone in between instead of snapping to the nearer note.",
        }),
        credit: "Assembled in Bayin from CHIMERA's tubes and lip valve.",
        build,
    }
}
