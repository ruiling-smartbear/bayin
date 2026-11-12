//! The airway physics: tubes of moving air, the openings between them, and the valves and jets
//! that set the air vibrating. All of it is CHIMERA's (github.com/jamesondh/chimera-vocal-synth,
//! MIT, Jameson Hodge), used unchanged. This file is the only one that names that crate.

pub use chimera::airway::network::{AirwayNetwork, PortSpec, Tube};
pub use chimera::creatures::build::{CMH2O, TubeArea, TubeOpts, fill_areas, lungs, tube};
pub use chimera::physics::{C_SOUND, WALL_SOFT};
pub use chimera::sources::valves::{LipReed, Whistle};

/// Transcendental math that gives the same bits on every machine.
pub mod math {
    pub use chimera::math::{PI, Rng, cbrt, exp, ln, pow, sqrt, tanh};
}
