pub mod debug;
pub mod grid;
pub mod query;

pub use debug::{CollisionDebugSystem, DebugConfig};
pub use grid::{Collider, CollisionGridSystem, CollisionLayer, SpatialGrid};
// query helpers are provided as methods on SpatialGrid, so no separate re-export needed
