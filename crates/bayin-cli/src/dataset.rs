//! Paired actions and sound, for learning how one turns into the other: random performances of
//! an instrument, each saved as audio with the actions that made it. See docs/DATA.md.
//!
//! A performance is a run of notes. At each note the varied parameters take new random values;
//! most notes are tongued (the gate closes for 40 ms first), some are slurred, and now and then
//! there is a rest.
use crate::{FS, setting, spec, wav};
use bayin::air::math::{Rng, exp, ln};
use bayin::{Kind, Player};

/// samples between action rows: 100 rows a second
const HOP: usize = 480;

struct Vary {
    key: &'static str,
    index: usize,
    lo: f64,
    hi: f64,
    log: bool,
}

pub fn run(a: &[String]) -> Result<(), String> {
    let (id, out_dir) = (a.first().ok_or(crate::USAGE)?, a.get(1).ok_or(crate::USAGE)?);
    let s = spec(id)?;
    let (mut episodes, mut seconds, mut seed) = (10usize, 10.0f64, 1u32);
    let mut vary: Vec<Vary> = Vec::new();
    let mut fixed: Vec<(usize, f64)> = Vec::new();
    let mut it = a[2..].iter();
    while let Some(x) = it.next() {
        let mut value = |name: &str| it.next().ok_or(format!("{name} needs a value"));
        match x.as_str() {
            "--episodes" => episodes = value(x)?.parse().map_err(|_| "--episodes: not a number")?,
            "--seconds" => seconds = value(x)?.parse().map_err(|_| "--seconds: not a number")?,
            "--seed" => seed = value(x)?.parse().map_err(|_| "--seed: not a number")?,
            "--vary" => {
                let v = value(x)?;
                let p: Vec<&str> = v.split(':').collect();
                let index = s.param(p[0]).ok_or_else(|| format!("{id} has no parameter {}", p[0]))?;
                let num = |i: usize| p.get(i).and_then(|v| v.parse::<f64>().ok()).ok_or(format!("--vary {v}: expected key:lo:hi[:log]"));
                vary.push(Vary { key: s.params[index].key, index, lo: num(1)?, hi: num(2)?, log: p.get(3) == Some(&"log") });
            }
            kv => fixed.push(setting(s, kv)?),
        }
    }
    if vary.is_empty() {
        vary = s.params.iter().enumerate().filter(|(_, p)| p.kind == Kind::Action).map(|(index, p)| Vary { key: p.key, index, lo: p.min, hi: p.max, log: p.log }).collect();
    }
    std::fs::create_dir_all(out_dir).map_err(|e| format!("{out_dir}: {e}"))?;
    let rows = (seconds * FS) as usize / HOP;
    let width = 2 * (1 + vary.len());
    let mut rng = Rng::new(seed);
    let draw = |rng: &mut Rng, v: &Vary| if v.log { exp(ln(v.lo) + rng.next() * (ln(v.hi) - ln(v.lo))) } else { v.lo + rng.next() * (v.hi - v.lo) };

    for ep in 0..episodes {
        let mut player = Player::new(s, FS, &fixed);
        let mut audio = vec![0f32; rows * HOP];
        let mut acts = vec![0f32; rows * width];
        // the performance: what is asked for now, when the next note starts, and the rows left
        // of a tongue's silence before the gate opens again
        let mut asked: Vec<f64> = vary.iter().map(|v| draw(&mut rng, v)).collect();
        let (mut gate, mut next_note, mut tongue) = (0.0, 0usize, 0usize);
        for r in 0..rows {
            if r == next_note {
                let rest = rng.next() < 0.12;
                let length = 0.12 + 0.78 * rng.next() * rng.next(); // mostly short notes, some long
                next_note = r + ((length * 100.0) as usize).max(8);
                if rest {
                    gate = 0.0;
                } else {
                    for (x, v) in asked.iter_mut().zip(&vary) {
                        *x = draw(&mut rng, v);
                    }
                    let slur = gate > 0.0 && rng.next() < 0.25;
                    if !slur {
                        tongue = 4;
                    }
                    gate = 1.0;
                }
                let set: Vec<(usize, f64)> = vary.iter().zip(&asked).map(|(v, x)| (v.index, *x)).collect();
                player.set(&set);
            }
            let g = if tongue > 0 { 0.0 } else { gate };
            tongue = tongue.saturating_sub(1);
            player.set_gate(g);
            player.render(&mut audio[r * HOP..(r + 1) * HOP]);
            let row = &mut acts[r * width..(r + 1) * width];
            row[0] = g as f32;
            row[1 + vary.len()] = player.gate() as f32;
            for (i, v) in vary.iter().enumerate() {
                row[1 + i] = asked[i] as f32;
                row[2 + vary.len() + i] = player.felt()[v.index] as f32;
            }
        }
        wav::write(&format!("{out_dir}/ep{ep:05}.f32"), &audio, FS as u32)?;
        wav::write(&format!("{out_dir}/ep{ep:05}.act.f32"), &acts, 100)?;
    }
    let mut cols = vec!["gate_asked".to_string()];
    cols.extend(vary.iter().map(|v| format!("{}_asked", v.key)));
    cols.push("gate_felt".to_string());
    cols.extend(vary.iter().map(|v| format!("{}_felt", v.key)));
    let list = |items: Vec<String>| items.join(", ");
    let meta = format!(
        "{{\n  \"instrument\": \"{id}\",\n  \"episodes\": {episodes},\n  \"seconds\": {seconds},\n  \"seed\": {seed},\n  \"sample_rate\": {FS},\n  \"action_rate\": 100,\n  \"columns\": [{}],\n  \"vary\": [{}],\n  \"fixed\": [{}]\n}}\n",
        list(cols.iter().map(|c| format!("\"{c}\"")).collect()),
        list(vary.iter().map(|v| format!("{{\"key\": \"{}\", \"lo\": {}, \"hi\": {}, \"log\": {}}}", v.key, v.lo, v.hi, v.log)).collect()),
        list(fixed.iter().map(|(i, x)| format!("{{\"key\": \"{}\", \"value\": {x}}}", s.params[*i].key)).collect()),
    );
    std::fs::write(format!("{out_dir}/meta.json"), meta).map_err(|e| format!("{out_dir}/meta.json: {e}"))?;
    println!("{episodes} episodes of {seconds} s → {out_dir}");
    Ok(())
}
