"""Measure a recording or a render, so the two can be compared.

    python3 tools/measure.py describe file.wav [seconds]   level, bands, partials at one moment
    python3 tools/measure.py notes file.wav                every steady note and its harmonics

Reads 16-bit WAV, or raw 48 kHz mono float32 (.f32). Needs numpy.
"""
import json, sys, wave
import numpy as np

BANDS = [(0, 250), (250, 700), (700, 1500), (1500, 3000), (3000, 6000), (6000, 12000)]

def load(path):
    if path.endswith('.f32'):
        return np.fromfile(path, dtype='<f4').astype(float), 48000
    w = wave.open(path); n, fs, ch = w.getnframes(), w.getframerate(), w.getnchannels()
    x = np.frombuffer(w.readframes(n), dtype='<i2').astype(float) / 32768
    return (x.reshape(-1, ch).mean(1) if ch > 1 else x), fs

def pitch(seg, fs, lo=140, hi=1000):
    """Fundamental (Hz) by autocorrelation, and how periodic the sound is (1 is perfectly)."""
    seg = seg - seg.mean()
    ac = np.fft.irfft(np.abs(np.fft.rfft(seg * np.hanning(len(seg)), 2 * len(seg))) ** 2)[:len(seg)]
    ac /= ac[0] + 1e-12
    a, b = int(fs / hi), int(fs / lo); k = a + int(np.argmax(ac[a:b]))
    y0, y1, y2 = ac[k - 1], ac[k], ac[k + 1]
    return fs / (k + 0.5 * (y0 - y2) / (y0 - 2 * y1 + y2 + 1e-12)), float(ac[k])

def harmonics(seg, fs, f0, count=10):
    """Levels of harmonics 1..count in dB re the strongest."""
    X = np.abs(np.fft.rfft(seg * np.hanning(len(seg)))); f = np.fft.rfftfreq(len(seg), 1 / fs); h = []
    for k in range(1, count + 1):
        m = (f > k * f0 * 0.975) & (f < k * f0 * 1.025)
        h.append(X[m].max() if m.any() and k * f0 < fs / 2 else 0.0)
    h = np.array(h)
    return [round(float(v), 1) for v in 20 * np.log10(h / (h.max() + 1e-30) + 1e-6)]

def describe(x, fs, at=None, n=32768):
    """Level, band energies and partials of n samples at `at` seconds (default: the loudest stretch)."""
    if at is None:
        e = np.convolve(x ** 2, np.ones(n) / n, 'valid'); i = int(np.argmax(e))
    else:
        i = int(at * fs)
    seg = x[i:i + n]; X = np.abs(np.fft.rfft(seg * np.hanning(len(seg)))); f = np.fft.rfftfreq(len(seg), 1 / fs); P = X ** 2
    rms = float(np.sqrt((seg ** 2).mean()))
    Y = X.copy(); Y[(f < 150) | (f > 6000)] = 0; top = Y.max(); partials = []
    for _ in range(24):
        k = int(np.argmax(Y))
        if Y[k] < top * 0.03: break
        partials.append((round(float(f[k]), 1), round(float(20 * np.log10(Y[k] / top)), 1))); Y[max(0, k - 8):k + 9] = 0
    return dict(at=i / fs, rms=rms, crest=float(np.abs(seg).max() / (rms + 1e-12)),
                bands=[round(float(10 * np.log10(P[(f >= a) & (f < b)].sum() / P.sum() + 1e-12)), 1) for a, b in BANDS],
                partials=sorted(partials))

def notes(x, fs, lo=180, hi=900, hop=0.02, win=0.06):
    """Steady notes: [{f0, at, seconds, rms, harm}]."""
    n = int(win * fs); track = []
    for i in range(0, len(x) - n, int(hop * fs)):
        seg = x[i:i + n]
        f0, clarity = pitch(seg, fs, lo, hi) if np.sqrt(((seg - seg.mean()) ** 2).mean()) > 0.02 else (0.0, 0.0)
        track.append(f0 if clarity > 0.6 else 0.0)
    out = []; i = 0
    while i < len(track):
        if track[i] <= 0: i += 1; continue
        j = i
        while j + 1 < len(track) and track[j + 1] > 0 and abs(1200 * np.log2(track[j + 1] / track[i])) < 60: j += 1
        seconds = (j - i + 1) * hop
        if seconds >= 0.25:
            seg = x[int((i * hop + 0.08) * fs):int((j * hop + win - 0.04) * fs)][:32768]; f0 = float(np.median(track[i:j + 1]))
            out.append(dict(f0=round(f0, 1), at=round(i * hop, 2), seconds=round(seconds, 2), rms=round(float(np.sqrt((seg ** 2).mean())), 3), harm=harmonics(seg, fs, f0, 12)))
        i = j + 1
    return out

if __name__ == '__main__':
    if len(sys.argv) < 3 or sys.argv[1] not in ('describe', 'notes'):
        sys.exit(__doc__)
    x, fs = load(sys.argv[2])
    if sys.argv[1] == 'notes':
        for r in notes(x, fs): print(json.dumps(r))
    else:
        d = describe(x, fs, float(sys.argv[3]) if len(sys.argv) > 3 else None)
        print('%.1f s; measured at %.2f s: rms %.3f, crest %.1f' % (len(x) / fs, d['at'], d['rms'], d['crest']))
        print('bands (dB) ' + '  '.join('%d-%d: %.1f' % (a, b, v) for (a, b), v in zip(BANDS, d['bands'])))
        print('partials (Hz:dB) ' + ' '.join('%.0f:%.0f' % p for p in d['partials']))
