//! The airway physics: tubes of moving air, the openings between them, and the valves and jets
//! that set the air vibrating. All of it is CHIMERA's (github.com/jamesondh/chimera-vocal-synth,
//! MIT, Jameson Hodge), used unchanged. This file is the only one that names that crate.

pub use chimera::airway::decimator::Decimator;
pub use chimera::airway::network::{AirwayNetwork, PortSpec, Tube};
pub use chimera::creatures::build::{CMH2O, TubeArea, TubeOpts, fill_areas, lungs, tube};
pub use chimera::physics::{C_SOUND, WALL_SOFT};
pub use chimera::sources::valves::{LipReed, Whistle};

/// Transcendental math that gives the same bits on every machine.
pub mod math {
    pub use chimera::math::{PI, Rng, cbrt, exp, ln, pow, sqrt, tanh};
}

/// The human voice: the whole model, its articulators and their limits, and its vowel table.
pub mod voice {
    pub use chimera::anatomy::human::{ArticKey, ArticPatch, BodyMix, N_ARTIC, NEUTRAL};
    pub use chimera::control::articulators::ARTIC_SPECS;
    pub use chimera::phonetics::planner::vowel_artic;
    pub use chimera::voice::human_voice::{HumanVoice, SourcePatch, oversample_for};
}

/// The creatures: their definitions and simulations, and the filter that brings an oversampled
/// simulation down to the output rate.
pub mod bestiary {
    pub use chimera::airway::decimator::Decimator;
    pub use chimera::creatures::types::Curve;
    pub use chimera::creatures::{CreatureDef, CreatureSim, creature_by_id};
}

/// A mammal's airway (windpipe, folds, throat, mouth, nose) and its fold model.
pub mod mammal {
    pub use chimera::creatures::mammal::{MammalAirway, MammalSpec, mammal_airway, mammal_noise, step_glottis};
}
