use super::*;
#[test]
fn compatibility_history_is_requested_only_for_the_lifetime_of_raw_subscribers() {
    let bus = EventBus::new(16);
    let projection = bus.subscribe_projection();
    assert!(!bus.history_requested());
    let raw = bus.subscribe_domain();
    assert!(bus.history_requested());
    let second = bus.clone().subscribe_domain();
    drop(raw);
    assert!(bus.history_requested());
    drop(second);
    assert!(!bus.history_requested());
    drop(projection);
}
