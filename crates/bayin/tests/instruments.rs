//! What every instrument must do, and what each one was fitted to do.
use bayin::{Player, Spec, instrument, instruments};

const FS: f64 = 48000.0;

/// `seconds` of a held note (the gate open), with these parameters set by key.
fn hold(spec: &'static Spec, seconds: f64, set: &[(&str, f64)]) -> Vec<f32> {
    let set: Vec<(usize, f64)> = set.iter().map(|(k, v)| (spec.param(k).unwrap_or_else(|| panic!("{} has no {k}", spec.id)), *v)).collect();
    let mut player = Player::new(spec, FS, &set);
    let mut out = vec![0f32; (seconds * FS) as usize];
    player.set_gate(1.0);
    player.render(&mut out);
    out
}

fn rms(x: &[f32]) -> f64 {
    (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len() as f64).sqrt()
}

/// The fundamental (Hz) by autocorrelation, between `lo` and `hi`.
fn pitch(x: &[f32], lo: f64, hi: f64) -> f64 {
    let x: Vec<f64> = x.iter().map(|v| *v as f64).collect();
    let ac = |lag: usize| (0..x.len() - lag).map(|i| x[i] * x[i + lag]).sum::<f64>();
    let (a, b) = ((FS / hi) as usize, (FS / lo) as usize);
    let k = (a..b).max_by(|i, j| ac(*i).total_cmp(&ac(*j))).unwrap();
    let (y0, y1, y2) = (ac(k - 1), ac(k), ac(k + 1));
    FS / (k as f64 + 0.5 * (y0 - y2) / (y0 - 2.0 * y1 + y2))
}

fn cents(f: f64, target: f64) -> f64 {
    1200.0 * (f / target).log2()
}

#[test]
fn every_instrument_sounds_within_full_scale_and_is_silent_before_the_gate_opens() {
    for spec in instruments() {
        let mut player = Player::new(spec, FS, &[]);
        let mut quiet = vec![0f32; 4800];
        player.render(&mut quiet);
        assert!(rms(&quiet) < 1e-6, "{} makes sound with the gate closed", spec.id);
        let x = hold(spec, 1.5, &[]);
        assert!(x.iter().all(|v| v.is_finite()), "{}", spec.id);
        let peak = x.iter().fold(0f32, |m, v| m.max(v.abs()));
        assert!(peak < 0.95, "{} peaks at {peak}", spec.id);
        assert!(rms(&x[36000..]) > 0.02, "{} is too quiet held: {}", spec.id, rms(&x[36000..]));
    }
}

#[test]
fn the_same_actions_give_the_same_samples_however_render_is_sliced() {
    for spec in instruments() {
        let whole = hold(spec, 0.5, &[]);
        let mut player = Player::new(spec, FS, &[]);
        player.set_gate(1.0);
        let mut pieces = vec![0f32; whole.len()];
        for block in pieces.chunks_mut(37) {
            player.render(block);
        }
        assert!(whole == pieces, "{} depends on the block size", spec.id);
    }
}

#[test]
fn every_phrase_plays_to_its_end_and_hands_the_parameters_back() {
    for spec in instruments() {
        for (i, phrase) in spec.phrases.iter().enumerate() {
            let mut player = Player::new(spec, FS, &[]);
            player.play(i);
            let end = phrase.keys.last().unwrap().t;
            let mut out = vec![0f32; ((end + 0.2) * FS) as usize];
            player.render(&mut out);
            assert!(!player.playing(), "{} {} is still playing", spec.id, phrase.id);
            assert!(rms(&out) > 0.005, "{} {} is silent", spec.id, phrase.id);
        }
    }
}

#[test]
fn the_bugle_sounds_a_g_bugles_notes_cleanly() {
    let bugle = instrument("bugle").unwrap();
    // asked for the real notes, dry; the fit is within 31 cents (see the spec's reference)
    for note in [394.2, 493.4, 594.5] {
        let x = hold(bugle, 1.0, &[("pitch", note), ("room", 0.0)]);
        let f = pitch(&x[24000..32192], 200.0, 900.0);
        assert!(cents(f, note).abs() < 40.0, "asked {note}, got {f:.1} Hz ({:+.0} cents)", cents(f, note));
    }
}

#[test]
fn the_air_horns_chimes_are_on_their_notes() {
    let horn = instrument("airhorn").unwrap();
    // the lowest chime alone, dry
    let x = hold(horn, 1.5, &[("chimes", 1.0), ("room", 0.0)]);
    let f = pitch(&x[48000..56192], 200.0, 500.0);
    assert!(cents(f, 311.13).abs() < 15.0, "lowest chime at {f:.1} Hz");
}
