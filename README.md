<p align="center"><img src="assets/logo.svg" width="128" alt="Bayin"></p>

<h1 align="center">Bayin 八音</h1>

<p align="center"><b>Instruments as physical models: actions in, sound out.</b></p>

<p align="center">Built on <a href="https://github.com/jamesondh/chimera-vocal-synth">CHIMERA</a> by Jameson Hodge, which simulates the air. See <a href="#credits">Credits</a>.</p>

Bayin simulates how an instrument makes sound. You give it what a player does (the pitch the
lips are set for, the breath, the tongue) and it computes the air moving through the
instrument, sample by sample. There are no recordings inside and no neural audio.

八音, "the eight sounds", is the old Chinese word for all instruments together.

It exists to make data for models that learn how actions become sound: any amount of audio,
every second of it paired with the actions that made it, from instruments whose design is
itself a set of numbers you can change.

```
   actions, 100 a second            design, fixed numbers
   pitch · breath · tongue          bore · mouthpiece · lips
            │                                │
            ▼                                ▼
      ┌──────────────────── instrument ────────────────────┐
      │  lips ─ cup ─ throat ─ backbore ─ bore ─ bell       │ ──▶  sound, 48 kHz
      └─────────────────────────────────────────────────────┘
```

## Instruments

| | How it makes sound | Model | Measured against | Fit |
|---|---|---|---|---|
| `voice` | vocal folds under a throat, mouth and nose shaped by tongue, jaw, lips and soft palate | CHIMERA's, unchanged | CHIMERA's own vowel table | five vowels within 2% of its formants |
| `songbird` `seal` `alligator` `elephant` | a syrinx, a seal's larynx, an alligator's roar, an elephant's rumble and trumpet | CHIMERA's, unchanged | | sample for sample the same as in CHIMERA |
| `bugle` | lips buzzing into a mouthpiece, a brass bore and a bell | assembled here from CHIMERA's parts | a real G bugle | three main notes within 31 cents, clean; harmonics within about 5 dB |
| `dog` | a larynx under a throat, a long mouth and a nose: bark, growl, howl, whine | assembled here from CHIMERA's parts | real barks, growls and a howl | lengths and pitches in the measured ranges; the bark is too bright |
| `airhorn` | five diaphragms, each into its own flaring horn | assembled here from CHIMERA's parts | nothing yet | each chime within 5 cents of its note |
| `steamwhistle` | steam blown across the mouths of three stopped bells | assembled here from CHIMERA's parts | nothing yet | |

`bayin list <id>` prints whose model an instrument is, its actions, its design, its phrases, the
recording it was measured against and how close it is.

## Quick start

```sh
cargo build --release

target/release/bayin list bugle
target/release/bayin play voice vowels.wav --phrase vowels
target/release/bayin play bugle fanfare.wav --phrase fanfare
target/release/bayin play bugle note.wav --seconds 2 pitch=494 pressure=55 room=0
```

A dataset of random performances, audio plus actions:

```sh
target/release/bayin dataset bugle data/bugle --episodes 100 --seconds 10 \
    --vary pitch:280:720:log --vary pressure:40:75 room=0
```

It renders about nine times faster than real time on one core, and the same seed gives the same
bytes on any machine. The format is in [docs/DATA.md](docs/DATA.md).

## Actions, not notes

MIDI says which note is wanted. It does not say how the player gets it, and it makes no sound.
Bayin starts one step earlier, at what the player's body does, and the note is whatever the
physics gives back.

| | MIDI | Bayin |
|---|---|---|
| What is written down | the note: pitch number, velocity, on, off | the action: lip setting, breath pressure, tongue |
| Where the sound comes from | a sampler or synth, added afterwards | the simulation itself |
| Can the note fail? | no | yes: it can crack to another harmonic, speak late, or not speak |
| Blowing harder | louder, if the synth says so | louder, brighter and sharper, because the air does that |
| The instrument | a preset name | numbers: bore, bell, mouthpiece, lips |

For a piano the two are nearly the same thing, because a key is a note. For anything blown or
bowed they are far apart, and that gap is what a model of the instrument has to learn.

## How it is put together

- An **instrument** is a physical model with two jobs: take the current actions, give the next
  sample. Its **spec** lists its parameters, each one an *action* (what the player does), a
  *design* value (what the instrument is: change it and it is another instrument) or the *space*
  it is heard in. A **player** plays one.
- The air inside the wind instruments (tubes, openings, lip valves, jets) is simulated by
  [CHIMERA](https://github.com/jamesondh/chimera-vocal-synth)'s airway engine, used unchanged
  and pinned to one commit. The `voice` and the four creatures are CHIMERA's own models; Bayin gives them the same
  sheet as the others. For the voice: 17 articulators, pitch and breath as actions, the body as its design. A
  single file, `air.rs`, is all that knows that crate.
- Instruments are grouped by how they make sound. Only `aerophones` (a column of air) has
  anything in it yet. Strings, skins and bars need their own physics.
- A design is not guessed. `tools/calibrate_bugle.py` searches for the one whose notes match a
  real recording's, and a test holds the result in place.

To add an instrument: [docs/ADDING.md](docs/ADDING.md).

## What is not right yet

- **Bugle.** Its lowest note (297 Hz) does not sound. Asked for a pitch between two notes, it
  gives a rough tone in between; a real bugle snaps to the nearer note. Its lowest main note is
  31 cents flat.
- **Voice.** A dataset of random articulator positions is babble, mostly not speech. Sung or
  spoken words (CHIMERA can plan them) are not connected yet. The vowel phrases use a man's
  vowel postures whatever the body.
- **Dog.** Its bark has about 10 dB too much energy between 1.5 and 3 kHz. Its airway is a
  guess at a medium dog. Its whine was not measured against anything.
- **Air horn and steam whistle** are not fitted to recordings. A real K5LA horn is much brighter
  between 700 and 3000 Hz.
- **The space and the wave steepening** are simple stages after the instrument's opening. They
  are not part of the airway simulation.
- Nobody has compared these with the recordings by ear in a controlled way.

## Related work

The [Synthesis ToolKit](https://github.com/thestk/stk) and
[Faust's physical modelling library](https://faustlibraries.grame.fr/libs/physmodels) have
physical models of many more instruments. Bayin differs in two ways: each design is fitted to a
real recording and says how close it got, and everything is arranged around writing down the
actions next to the sound.

## Credits

Bayin is a layer on other people's work.

- **[CHIMERA](https://github.com/jamesondh/chimera-vocal-synth)**, by Jameson Hodge (MIT). Every
  sound here comes out of CHIMERA's simulation of air in tubes. The human voice, the songbird,
  the seal, the alligator and the elephant are CHIMERA's models outright, with their parameters
  and calls, used unchanged. The bugle, the air horn, the steam whistle and the dog are assembled
  here from CHIMERA's tubes, valves, fold model and mammal airway.
- **The recordings** each instrument is measured against belong to the people who made them and
  shared them on Wikimedia Commons: [references/SOURCES.md](references/SOURCES.md) lists every
  one with its author and license. None of the audio is in this repository.
- **The idea** of an instrument as actions that become sound, to be learned by a world model, is
  from [Music-JEPA](https://arxiv.org/abs/2607.22000) (Wang, Fang and LeCun, 2026).

What Bayin adds: one sheet for every instrument (actions, design, space), a player, the
instruments it assembles, fitting designs to recordings and saying how close they are, and
writing the actions down next to the sound. [NOTICE](NOTICE) has the full account.

## License

Bayin's own code is MIT ([LICENSE](LICENSE)). CHIMERA is MIT, © Jameson Hodge
([LICENSES/CHIMERA-MIT.txt](LICENSES/CHIMERA-MIT.txt)).
