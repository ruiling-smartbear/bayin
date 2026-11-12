"""The shape of calls in a recording: each sound's length, pitch and how much of it is clear.

    python3 tools/contour.py file.wav [lowest_hz highest_hz]

For sounds that are not steady notes (barks, growls, howls). The recording is scaled to a common
peak first, so a quiet one and a loud one are measured alike.
"""
import sys
import numpy as np
from measure import BANDS, describe, load, pitch

def contour(path, lo=150, hi=1500, win=0.04, hop=0.02, floor=0.03):
    x, fs = load(path); x = x / (np.abs(x).max() + 1e-9) * 0.8
    n = int(win * fs); events = []; cur = None
    for i in range(0, len(x) - n, int(hop * fs)):
        seg = x[i:i + n]; t = i / fs
        if np.sqrt((seg ** 2).mean()) < floor:
            if cur: events.append(cur); cur = None
            continue
        cur = cur or dict(start=t, end=t, pitches=[], frames=0)
        cur['end'] = t + hop; cur['frames'] += 1
        f0, clarity = pitch(seg, fs, lo, hi)
        if clarity > 0.5: cur['pitches'].append(f0)
    if cur: events.append(cur)
    return [e for e in events if e['end'] - e['start'] >= 0.06], describe(x, fs)

if __name__ == '__main__':
    if len(sys.argv) < 2: sys.exit(__doc__)
    lo, hi = (float(sys.argv[2]), float(sys.argv[3])) if len(sys.argv) > 3 else (150, 1500)
    events, d = contour(sys.argv[1], lo, hi)
    print('%d sounds; bands (dB) ' % len(events) + '  '.join('%d-%d: %.0f' % (a, b, v) for (a, b), v in zip(BANDS, d['bands'])))
    for e in events:
        p = np.array(e['pitches'] or [0.0])
        print('  %.2f–%.2f s (%.2f s): pitch median %.0f, %.0f–%.0f Hz, clear %d%% of the time' % (e['start'], e['end'], e['end'] - e['start'], np.median(p), np.percentile(p, 10), np.percentile(p, 90), 100 * len(e['pitches']) / e['frames']))
