"""Fit the bugle's design to a real G bugle.

    python3 tools/calibrate_bugle.py [random_samples] [refine_rounds]

Asks for: each natural note on pitch, speaking within a tenth of a second, sounding steadily and
cleanly, as loud as the others, with the recording's harmonic levels; and a tongued fanfare in
which every note lands. Everything is measured dry (room=0). Starts from the design the library
ships, tries random ones, then narrows in on the best. Prints the best design; put it in
`DESIGN` and `BREATH` in crates/bayin/src/aerophones/bugle.rs.

The reference is references/assembly (see references/SOURCES.md); its notes, measured with
`tools/measure.py notes`, are in REF below. Needs numpy and a release build (cargo build --release).
"""
import json, os, subprocess, sys, tempfile
from concurrent.futures import ThreadPoolExecutor
import numpy as np
from measure import harmonics, pitch

BIN = os.environ.get('BAYIN', os.path.join(os.path.dirname(__file__), '..', 'target', 'release', 'bayin'))
FS = 48000
# the real bugle's notes: harmonics 1..10 in dB re the strongest
REF = {
    394.2: [-5.1, -12.2, 0.0, -0.8, -11.6, -10.8, -9.6, -10.8, -18.7, -18.7],
    493.4: [-15.0, 0.0, -0.3, -6.7, -5.4, -11.7, -10.4, -19.8, -16.7, -27.8],
    594.5: [-7.2, 0.0, -4.5, -11.5, -17.2, -18.7, -21.4, -29.9, -35.2, -35.9],
}
# the search's variables and their ranges ("pressure" is the breath the design is fitted at)
RANGES = {
    'pressure': (18, 70), 'cup': (0.4, 2.8), 'throat': (0.06, 0.28), 'backbore': (3.5, 13), 'bore': (0.7, 1.4),
    'flareStart': (0.4, 0.82), 'flareExp': (1.3, 6.5), 'bell': (35, 200), 'length': (136, 162),
    'lipBelow': (0.4, 0.98), 'lipDamping': (0.11, 0.35), 'lipWidth': (0.5, 2.5), 'lipPush': (0.03, 0.7),
    'lipRest': (-0.035, 0.015), 'lean': (0.0, 0.22),
}
# the fanfare the library ships: (note asked for, beats of 0.3 s)
TUNE = [(394, 1), (494, 1), (594, 2), (594, .5), (494, .5), (594, 1), (394, 1), (494, .5), (594, .5), (494, 1), (594, 4)]

def render(design, how):
    with tempfile.NamedTemporaryFile(suffix='.f32', delete=False) as f:
        out = f.name
    subprocess.run([BIN, 'play', 'bugle', out, 'room=0'] + how + [f'{k}={v}' for k, v in design.items()], check=True, stdout=subprocess.DEVNULL)
    x = np.fromfile(out, dtype='<f4').astype(float); os.unlink(out)
    return x

def note(design, target):
    x = render(design, ['--seconds', '1.1', f'pitch={target}'])
    early, late = x[int(0.2 * FS):int(0.4 * FS)], x[int(0.42 * FS):int(0.42 * FS) + 16384]
    level = float(np.sqrt((late ** 2).mean()))
    if not np.isfinite(level) or level < 1e-4:
        return dict(ok=False)
    f0, clarity = pitch(late[:8192], FS)
    fq, cq = pitch(x[int(0.10 * FS):int(0.10 * FS) + 4096], FS)
    return dict(ok=bool(level > 0.6 * np.sqrt((early ** 2).mean()) and clarity > 0.5), level=level, f0=float(f0),
                cents=float(1200 * np.log2(f0 / target)), clarity=clarity,
                quick=bool(abs(1200 * np.log2(fq / target)) < 40 and cq > 0.6), harm=harmonics(late, FS, f0))

def note_cost(m, target):
    if not m['ok']: return 60.0
    if abs(m['cents']) > 70: return 40.0 + min(20.0, abs(m['cents']) / 100)  # locked onto another resonance
    ref, got = np.maximum(REF[target], -30), np.maximum(m['harm'], -30)
    return float(np.abs(ref - got).mean()) + abs(m['cents']) / 10 + (0 if m['quick'] else 5) + 60 * max(0.0, 0.97 - m['clarity'])

def fanfare_error(design):
    """Mean pitch error (cents) over the fanfare's notes, each measured in its second half."""
    x = render(design, ['--phrase', 'fanfare']); t = 0.03; errs = []
    for asked, beats in TUNE:
        dur = beats * 0.3; a = int((t + 0.5 * dur) * FS); seg = x[a:a + max(2048, int(0.4 * dur * FS))][:4096]; t += dur
        if len(seg) < 2048 or np.sqrt((seg ** 2).mean()) < 1e-4:
            errs.append(150.0); continue
        f0, clarity = pitch(seg, FS)
        errs.append(min(150.0, abs(1200 * np.log2(f0 / asked)) + (60 if clarity < 0.9 else 0)))
    return float(np.mean(errs))

def cost(design):
    try:
        ms = {t: note(design, t) for t in REF}; fan = fanfare_error(design)
    except Exception as e:  # a design the model cannot run
        return 999.0, design, {'error': str(e)}
    c = sum(note_cost(ms[t], t) for t in REF) / len(REF) + fan / 8
    levels = [20 * np.log10(m['level']) for m in ms.values() if m['ok']]
    if len(levels) == len(REF):  # the real bugle's notes are within 4 dB of each other
        c += max(0.0, max(levels) - min(levels) - 4) / 3
    return c, design, dict(notes=ms, fanfare_cents=fan)

def shipped():
    """The design the library ships, read from `bayin list bugle`."""
    out = subprocess.run([BIN, 'list', 'bugle'], check=True, capture_output=True, text=True).stdout
    d = {}
    for line in out.splitlines():
        w = line.split()
        if w and w[0] in RANGES and '(default' in line:
            d[w[0]] = float(line.split('(default')[1].strip(' )'))
    return d

def search(samples, rounds, rng=np.random.default_rng(11)):
    jobs = [shipped()] + [{k: float(rng.uniform(*r)) for k, r in RANGES.items()} for _ in range(samples)]
    with ThreadPoolExecutor(os.cpu_count() or 4) as pool:
        best = sorted(pool.map(cost, jobs), key=lambda r: r[0])
        print('start: best %.2f (%d of %d designs sound)' % (best[0][0], sum(r[0] < 40 for r in best), len(best)), flush=True)
        for rnd in range(rounds):
            spread = 0.08 * 0.8 ** rnd
            jobs = [{k: float(np.clip(t[k] + rng.normal() * spread * (hi - lo), lo, hi)) for k, (lo, hi) in RANGES.items()} for _, t, _ in best[:8] for _ in range(14)]
            best = sorted(best[:24] + list(pool.map(cost, jobs)), key=lambda r: r[0])
            print('round %d: best %.2f' % (rnd, best[0][0]), flush=True)
    return best[0]

if __name__ == '__main__':
    c, design, m = search(int(sys.argv[1]) if len(sys.argv) > 1 else 200, int(sys.argv[2]) if len(sys.argv) > 2 else 8)
    print('\ncost %.2f, fanfare off by %.0f cents on average' % (c, m['fanfare_cents']))
    order = ['cup', 'throat', 'backbore', 'bore', 'flareStart', 'flareExp', 'bell', 'length', 'lipBelow', 'lipDamping', 'lipWidth', 'lipPush', 'lipRest', 'lean']
    print('DESIGN = [' + ', '.join('%.4f' % design[k] for k in order) + ']\nBREATH = %.1f' % design['pressure'])
    for t, n in m['notes'].items():
        print('%.1f Hz: ' % t + ('%.1f Hz (%+.0f cents), clarity %.2f, quick %s\n   got %s\n   ref %s' % (n['f0'], n['cents'], n['clarity'], n['quick'], n['harm'], REF[t]) if n['ok'] else 'does not sound'))
