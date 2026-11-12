//! Bayin (八音): instruments as physical models. Each one turns actions (a pitch asked of the
//! lips, a breath, a valve pulled) into sound, sample by sample, by simulating how the real
//! thing makes it. The name is the old Chinese word for all instruments together, the eight
//! sounds.
//!
//! - [`Instrument`]: the physical model. [`Spec`]: its sheet (parameters, phrases, the recording
//!   it was measured against). [`Player`]: plays one.
//! - [`aerophones`]: instruments of a column of air.
//! - [`air`]: the airway physics, which is CHIMERA's; nothing else here knows that crate.
//! - [`parts`]: physics shared between instruments that the airway does not cover.
//!
//! Everything is deterministic: the same actions give the same samples, on any machine.

pub mod aerophones;
pub mod air;
pub mod instrument;
pub mod parts {
    pub mod space;
    pub mod steepen;
}
pub mod player;

pub use instrument::{BLOCK, Curve, Family, Instrument, Key, Kind, Param, Phrase, Reference, Spec};
pub use player::Player;

use std::sync::LazyLock;

static INSTRUMENTS: LazyLock<Vec<Spec>> = LazyLock::new(|| vec![aerophones::voice::spec(), aerophones::bugle::spec(), aerophones::air_horn::spec(), aerophones::steam_whistle::spec()]);

/// Every instrument.
pub fn instruments() -> &'static [Spec] {
    &INSTRUMENTS
}

pub fn instrument(id: &str) -> Option<&'static Spec> {
    INSTRUMENTS.iter().find(|s| s.id == id)
}
