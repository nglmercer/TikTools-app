use tiktools_app_update::{Channel, InstallationType, Phase, UpdateState};

#[test]
fn duplicate_operations_and_invalid_transitions_are_rejected() {
    let state = UpdateState::new("4.0.0", Channel::Stable, InstallationType::Unknown);
    assert!(state.transition(Phase::Checking).is_ok());
    assert!(state.transition(Phase::Checking).is_err());
    assert!(state.transition(Phase::Installing).is_err());
    assert!(state.transition(Phase::Available).is_ok());
    assert!(!state.snapshot().install_supported);
}

#[test]
fn only_one_concurrent_check_is_admitted() {
    let state = std::sync::Arc::new(UpdateState::new(
        "4.0.0",
        Channel::Stable,
        InstallationType::Unknown,
    ));
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
    let threads: Vec<_> = (0..8)
        .map(|_| {
            let state = state.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                state.transition(Phase::Checking).is_ok()
            })
        })
        .collect();
    let admitted = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .filter(|ok| *ok)
        .count();
    assert_eq!(admitted, 1);
}
