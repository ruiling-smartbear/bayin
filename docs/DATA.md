# The dataset format

`bayin dataset <id> <dir>` writes, for each episode `N`:

| File | What |
|---|---|
| `epNNNNN.f32` | the sound: 48 kHz, mono, 32-bit float, little-endian, no header |
| `epNNNNN.act.f32` | the actions: 100 rows a second, 32-bit float, little-endian, no header |
| `meta.json` | the instrument, the seed, the columns, what was varied and what was fixed |

Row `r` of the actions is the state at `r / 100` seconds, held for the 480 samples that follow.
Its columns, named in `meta.json`:

1. `gate_asked`: 0 or 1, the gate the performer asked for
2. `<key>_asked`: one per varied parameter, the value the performer asked for
3. `gate_felt`: the gate as the instrument felt it (it opens and closes over a few milliseconds)
4. `<key>_felt`: one per varied parameter, as the instrument felt it (smoothed)

"Asked" is the action. "Felt" is given so a model can be told either.

## A performance

A run of notes, each 0.12 to 0.9 s, most of them short. At each note every varied parameter
takes a new value, uniform in its range (or uniform in its logarithm, for `:log`). Three notes
in four are tongued: the gate closes for 40 ms first. The rest are slurred. One note in eight is
a rest.

Without `--vary`, every action parameter varies over its whole range. `key=value` holds a
parameter fixed for all episodes; `room=0` gives the dry sound at the instrument.

## Reading it

```python
import json, numpy as np
meta = json.load(open('data/bugle/meta.json'))
audio = np.fromfile('data/bugle/ep00000.f32', dtype='<f4')
actions = np.fromfile('data/bugle/ep00000.act.f32', dtype='<f4').reshape(-1, len(meta['columns']))
```
