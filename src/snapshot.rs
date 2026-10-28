//! The snapshot compiled into the binary; the fallback when Yahoo is unreachable.

use std::sync::OnceLock;

use crate::model::Snapshot;

static SNAPSHOT: OnceLock<Snapshot> = OnceLock::new();

pub fn embedded() -> &'static Snapshot {
    SNAPSHOT.get_or_init(|| {
        serde_json::from_str(include_str!("../data/snapshot.json"))
            .expect("data/snapshot.json is valid")
    })
}
