# Adding an instrument

1. **Find how it makes sound**, and what part of that the airway engine already does. A wind
   instrument is a pressure supply, a valve or a jet, tubes, and an opening: see
   `aerophones/bugle.rs` (lips), `air_horn.rs` (a diaphragm) and `steam_whistle.rs` (a jet).
2. **Write the model**: a struct that implements `Instrument` (`control`, `settle`, `sample`) and
   a `spec()` that lists its parameters. Mark each parameter an action, a design value, or
   space. Give it a `room` parameter so it can be rendered dry. Add it to the list in `lib.rs`.
   Only `air.rs` may name the CHIMERA crate; add what you need there.
3. **Get a real recording** whose license lets you measure it. Add it to `references/fetch.sh`
   and `references/SOURCES.md`. Do not commit the audio.
4. **Measure both** with `tools/measure.py notes` and `describe`: the notes, their harmonics,
   the bands.
5. **Fit the design by search**, as `tools/calibrate_bugle.py` does: say what a good design is
   (on pitch, steady, clean, the recording's harmonics, a phrase in which every note lands) and
   let it look. Measure dry.
6. **Hold it in place**: a test for what it was fitted to do, and in the spec's `reference`,
   what was matched and what is still wrong.

Keep the sound within ±0.7 at the loudest setting (set `gain`), and check the render is the same
however it is sliced: `cargo test` checks both for every instrument.
