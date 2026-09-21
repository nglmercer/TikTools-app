//! Parity: the typed client must cover the control registry exactly.
//!
//! `ControlApi::register_all` is the single source of truth for the
//! control surface. This test fails CI when a registered method has no
//! typed wrapper, or when the client covers a name the registry does
//! not know (renamed or removed RPC method).

use std::collections::BTreeSet;

use tiktools_client::TikToolsClient;
use tiktools_control_api::{ControlApi, ControlRouter};

#[test]
fn client_covers_every_registered_method() {
    let mut router = ControlRouter::new();
    ControlApi::register_all(&mut router);
    let metadata = router.metadata();
    let registered: BTreeSet<&str> = metadata.iter().map(|meta| meta.name.as_str()).collect();
    let covered: BTreeSet<&str> = TikToolsClient::covered_methods().into_iter().collect();
    assert!(
        !registered.is_empty(),
        "registry must not be empty; register_all regressed"
    );
    let missing: Vec<&str> = registered.difference(&covered).copied().collect();
    let extra: Vec<&str> = covered.difference(&registered).copied().collect();
    assert!(
        missing.is_empty(),
        "typed client is missing methods: {missing:?}"
    );
    assert!(
        extra.is_empty(),
        "typed client covers unregistered methods: {extra:?}"
    );
    assert_eq!(
        registered.len(),
        covered.len(),
        "coverage count drifted from the registry"
    );
}
