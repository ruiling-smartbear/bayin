//! `bayin`: list the instruments, play one to a file, or render a dataset of random performances.
mod dataset;
mod wav;

use bayin::{Kind, Player, Spec};

const FS: f64 = 48000.0;

const USAGE: &str = "usage:
  bayin list [id]
  bayin play <id> <out.wav|out.f32> [--seconds s] [--phrase id] [key=value ...]
  bayin dataset <id> <out_dir> [--episodes n] [--seconds s] [--seed k] [--vary key:lo:hi[:log] ...] [key=value ...]

play holds the gate open for --seconds (1.5), or plays a phrase; a second and a half of release
follows. A .f32 output is raw 32-bit float samples; anything else is a 16-bit WAV. Both 48 kHz mono.
dataset varies every action parameter over its whole range unless --vary names the ones to vary.";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("list") => list(args.get(1)),
        Some("play") => play(&args[1..]),
        Some("dataset") => dataset::run(&args[1..]),
        _ => Err(USAGE.to_string()),
    };
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

pub fn spec(id: &str) -> Result<&'static Spec, String> {
    bayin::instrument(id).ok_or_else(|| format!("no instrument {id}; have {}", bayin::instruments().iter().map(|s| s.id).collect::<Vec<_>>().join(", ")))
}

/// `key=value` → (parameter index, value).
pub fn setting(spec: &Spec, kv: &str) -> Result<(usize, f64), String> {
    let (k, v) = kv.split_once('=').ok_or_else(|| format!("expected key=value, got {kv}"))?;
    let i = spec.param(k).ok_or_else(|| format!("{} has no parameter {k}", spec.id))?;
    Ok((i, v.parse().map_err(|_| format!("{kv}: not a number"))?))
}

fn list(id: Option<&String>) -> Result<(), String> {
    let Some(id) = id else {
        for s in bayin::instruments() {
            println!("{:14} {}", s.id, s.blurb);
        }
        return Ok(());
    };
    let s = spec(id)?;
    println!("{} ({})\n{}\n", s.name, s.id, s.blurb);
    for (kind, title) in [(Kind::Action, "Actions"), (Kind::Design, "Design"), (Kind::Space, "Space")] {
        let ps: Vec<_> = s.params.iter().filter(|p| p.kind == kind).collect();
        if ps.is_empty() {
            continue;
        }
        println!("{title}");
        for p in ps {
            println!("  {:12} {:28} {} to {} {}  (default {})", p.key, p.label, p.min, p.max, p.unit, p.default);
        }
    }
    println!("Phrases\n  {}", s.phrases.iter().map(|p| p.id).collect::<Vec<_>>().join(", "));
    match &s.reference {
        Some(r) => println!("Measured against\n  {}\n  {} ({})\n  {}", r.title, r.url, r.license, r.fit),
        None => println!("Measured against\n  no recording yet"),
    }
    Ok(())
}

fn play(a: &[String]) -> Result<(), String> {
    let (id, out) = (a.first().ok_or(USAGE)?, a.get(1).ok_or(USAGE)?);
    let s = spec(id)?;
    let mut seconds = 1.5;
    let mut phrase = None;
    let mut set = Vec::new();
    let mut it = a[2..].iter();
    while let Some(x) = it.next() {
        match x.as_str() {
            "--seconds" => seconds = it.next().and_then(|v| v.parse().ok()).ok_or("--seconds needs a number")?,
            "--phrase" => {
                let name = it.next().ok_or("--phrase needs an id")?;
                phrase = Some(s.phrase(name).ok_or_else(|| format!("{id} has no phrase {name}"))?);
            }
            kv => set.push(setting(s, kv)?),
        }
    }
    let mut player = Player::new(s, FS, &set);
    let hold = match phrase {
        Some(p) => s.phrases[p].keys.last().map_or(0.0, |k| k.t),
        None => seconds,
    };
    let mut audio = vec![0f32; ((hold + 1.5) * FS) as usize];
    match phrase {
        Some(p) => player.play(p),
        None => player.set_gate(1.0),
    }
    let release = (hold * FS) as usize;
    for (i, block) in audio.chunks_mut(128).enumerate() {
        if phrase.is_none() && i * 128 >= release {
            player.set_gate(0.0);
        }
        player.render(block);
    }
    wav::write(out, &audio, FS as u32)?;
    let peak = audio.iter().fold(0f32, |m, x| m.max(x.abs()));
    println!("{out}: {:.1} s, peak {peak:.3}", audio.len() as f64 / FS);
    Ok(())
}
