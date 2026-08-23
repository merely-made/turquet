// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

//! Compatibility surface for the inherited astro-rust API.
//!
//! These modules preserve the original anonymous `f64` contracts while
//! consumers migrate. New code should use Turquet's typed primary modules.

pub mod aberr {
    pub use aberr::*;
}
pub mod angle {
    pub use angle::*;
}
pub mod asteroid {
    pub use asteroid::*;
}
pub mod atmos {
    pub use atmos::*;
}
pub mod binary_star {
    pub use binary_star::*;
}
pub mod consts {
    pub use consts::*;
}
pub mod coords {
    pub use coords::*;
}
pub mod ecliptic {
    pub use ecliptic::*;
}
pub mod interpol {
    pub use interpol::*;
}
pub mod lunar {
    pub use lunar::*;
}
pub mod misc {
    pub use misc::*;
}
pub mod nutation {
    pub use nutation::*;
}
pub mod orbit {
    pub use orbit::*;
}
pub mod parallax {
    pub use parallax::*;
}
pub mod planet {
    pub use planet::*;
}
pub mod pluto {
    pub use pluto::*;
}
pub mod precess {
    pub use precess::*;
}
pub mod star {
    pub use star::*;
}
pub mod sun {
    pub use sun::*;
}
pub mod time {
    pub use time::*;
}
pub mod transit {
    pub use transit::*;
}
pub mod util {
    pub use util::*;
}

/// Anonymous-scalar wrappers for the first Turquet-era apparent pipeline.
pub mod apparent {
    pub use apparent::{ApparentBody, ApparentError, APPARENT_BODIES};
    pub use hifitime::Epoch;

    pub fn jde_tt_frm_epoch(epoch: Epoch) -> f64 {
        ::apparent::legacy_jde_tt_frm_epoch(epoch)
    }

    pub fn jde_tt_frm_utc(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: f64,
    ) -> Result<f64, ApparentError> {
        ::apparent::legacy_jde_tt_frm_utc(year, month, day, hour, minute, second)
    }

    pub fn geocent_apparent_ecl_pos(
        body: &ApparentBody,
        jde_tt: f64,
    ) -> Result<(f64, f64), ApparentError> {
        ::apparent::legacy_geocent_apparent_ecl_pos(body, jde_tt)
    }

    pub fn is_retrograde(body: &ApparentBody, jde_tt: f64) -> Result<bool, ApparentError> {
        ::apparent::legacy_is_retrograde(body, jde_tt)
    }
}
