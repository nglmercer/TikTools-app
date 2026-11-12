use super::*;
#[test]
fn persisted_highest_score_selection_enforces_tie_policy() {
    let eligible = vec![
        Candidate {
            key: "a".into(),
            weight: 5,
        },
        Candidate {
            key: "b".into(),
            weight: 5,
        },
    ];
    let mut selection = Selection {
        mode: SelectionMode::HighestScore,
        tie_policy: TiePolicy::Stable,
        eligible,
        winner: Some("b".into()),
    };
    assert_eq!(selection.validate(), Err(Error::InvalidCandidate));
    selection.winner = Some("a".into());
    assert_eq!(selection.validate(), Ok(()));
    selection.tie_policy = TiePolicy::NoWinner;
    assert_eq!(selection.validate(), Err(Error::InvalidCandidate));
    selection.winner = None;
    assert_eq!(selection.validate(), Ok(()));
}
#[test]
fn reveal_projection_keeps_winner_and_omitted_weight() {
    let selection = Selection {
        mode: SelectionMode::WeightedRandom,
        tie_policy: TiePolicy::Stable,
        eligible: (0..1000)
            .map(|i| Candidate {
                key: format!("user-{i:04}"),
                weight: 1,
            })
            .collect(),
        winner: Some("user-0999".into()),
    };
    let record = ResultRecord {
        schema_version: 1,
        competition_id: "c".into(),
        generation: 1,
        ranking_id: "r".into(),
        ranking_revision: 1,
        finalized_at: 1,
        selection,
    };
    let reveal = record.reveal_snapshot().unwrap();
    assert_eq!(reveal.eligible_count, 1000);
    assert_eq!(reveal.selection.eligible.len(), 256);
    assert_eq!(reveal.other_weight, 744);
    assert!(reveal
        .selection
        .eligible
        .iter()
        .any(|entry| entry.key == "user-0999"));
    assert_eq!(record.selection.eligible.len(), 1000);
}
#[test]
fn command_index_is_scoped_reusable_and_updates_atomically() {
    use crate::participants::CommandIndex;
    let option = |id: &str, command: &str| OptionDefinition {
        id: id.into(),
        label: id.into(),
        commands: vec![command.into()],
        gift_ids: vec![],
        image_url: None,
        on_win: None,
    };
    let mut definition = Definition {
        schema_version: 1,
        id: "a".into(),
        name: "Poll".into(),
        kind: CompetitionKind::Poll,
        ranking_id: "r".into(),
        options: vec![option("yes", "!Yes")],
        selection_mode: SelectionMode::HighestScore,
        tie_policy: TiePolicy::Stable,
        duration_ms: None,
        vote_limit: 1,
        allow_switch: false,
        minimum_participants: 1,
        target_score: None,
        on_user_win: None,
    };
    let mut index = CommandIndex::default();
    index.register(&definition, "creator", "room").unwrap();
    index
        .register_sources("a", "creator", "room", &["tiktok.like", "tiktok.share"])
        .unwrap();
    assert_eq!(
        index.matching_source("creator", "room", "tiktok.like"),
        vec!["a"]
    );
    assert!(index
        .matching_source("creator", "other", "tiktok.like")
        .is_empty());
    assert!(index
        .register_sources("a", "creator", "room", &["unknown.source"])
        .is_err());
    assert_eq!(
        index.matching_source("creator", "room", "tiktok.like"),
        vec!["a"]
    );
    definition.id = "b".into();
    index.register(&definition, "creator", "room").unwrap();
    let matches = index.matching("creator", "room", " !YES ");
    assert_eq!(
        matches
            .iter()
            .map(|r| r.competition_id.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b"]
    );
    assert!(index.matching("creator", "other", "!yes").is_empty());
    assert!(index.matching("other", "room", "!yes").is_empty());
    assert!(index.matching("creator", "room", "!yes extra").is_empty());
    definition.options.push(option("duplicate", "!YES"));
    assert!(index.register(&definition, "creator", "room").is_err());
    assert_eq!(index.matching("creator", "room", "!yes").len(), 2);
    definition.options = (0..33)
        .map(|option_id| OptionDefinition {
            id: format!("o{option_id}"),
            label: format!("Option {option_id}"),
            commands: (0..64)
                .map(|command| format!("!{option_id}-{command}"))
                .collect(),
            gift_ids: vec![],
            image_url: None,
            on_win: None,
        })
        .collect();
    assert_eq!(
        index.register(&definition, "creator", "room"),
        Err(Error::Capacity)
    );
    assert_eq!(index.matching("creator", "room", "!yes").len(), 2);
    definition.options = vec![option("yes", "!Yes")];
    index.register(&definition, "creator", "new-room").unwrap();
    assert_eq!(index.matching("creator", "room", "!yes").len(), 1);
    assert_eq!(
        index.matching("creator", "new-room", "!yes")[0].competition_id,
        "b"
    );
    assert!(index.remove("a"));
    assert!(index
        .matching_source("creator", "room", "tiktok.like")
        .is_empty());
    assert!(index
        .matching_source("creator", "room", "tiktok.share")
        .is_empty());
    assert!(index.matching("creator", "room", "!yes").is_empty());
}
#[test]
fn rejects_ambiguous_option_routes() {
    let option = |id: &str, command: &str| OptionDefinition {
        id: id.into(),
        label: id.into(),
        commands: vec![command.into()],
        gift_ids: vec![],
        image_url: None,
        on_win: None,
    };
    let mut definition = Definition {
        schema_version: 1,
        id: "poll".into(),
        name: "Poll".into(),
        kind: CompetitionKind::Poll,
        ranking_id: "r".into(),
        options: vec![option("a", "!Vote"), option("b", "!vote")],
        selection_mode: SelectionMode::HighestScore,
        tie_policy: TiePolicy::Stable,
        duration_ms: Some(1000),
        vote_limit: 1,
        allow_switch: false,
        minimum_participants: 1,
        target_score: None,
        on_user_win: None,
    };
    assert!(definition.validate().is_err());
    definition.options[1].commands[0] = "!other".into();
    definition.options[0].gift_ids.push("42".into());
    definition.options[1].gift_ids.push("42".into());
    assert!(definition.validate().is_err());
    definition.options[1].gift_ids.clear();
    assert!(definition.validate().is_ok());
}
#[test]
fn durable_lifecycle_freezes_remaining_time_and_rejects_late_pause() {
    let mut state = LifecycleState::default();
    state.transition(Transition::Start, 100, Some(50)).unwrap();
    assert_eq!(state.generation, 1);
    assert_eq!(
        state.transition(Transition::Pause, 150, Some(50)),
        Err(Error::InvalidTransition)
    );
    state.transition(Transition::Pause, 120, Some(50)).unwrap();
    assert_eq!(state.remaining_ms, Some(30));
    state.transition(Transition::Resume, 500, Some(50)).unwrap();
    assert_eq!(state.deadline_ms, Some(530));
    assert_eq!(
        state.transition(Transition::Expire, 529, None),
        Err(Error::InvalidTransition)
    );
    state.transition(Transition::Expire, 530, None).unwrap();
    assert_eq!(state.lifecycle, Lifecycle::Finalizing);
    assert_eq!(
        state.transition(Transition::Cancel, 531, None),
        Err(Error::InvalidTransition)
    );
}
#[test]
fn lifecycle_and_participation() {
    let mut c = Competition::new(["a".into(), "b".into()], 2, 2, true).unwrap();
    c.start(10, Some(100)).unwrap();
    c.vote(11, "u", "a").unwrap();
    c.vote(12, "u", "b").unwrap();
    c.vote(13, "u", "a").unwrap();
    assert_eq!(c.selected_option(13, "u"), Some("a"));
    assert_eq!(c.vote(14, "u", "a"), Err(Error::VoteLimit));
    c.pause(20).unwrap();
    assert_eq!(c.selected_option(21, "u"), None);
    c.resume(100).unwrap();
    assert!(!c.is_due(189));
    assert!(c.is_due(190));
    assert_eq!(c.selected_option(190, "u"), None);
    c.begin_finalizing().unwrap();
    c.finish().unwrap();
    assert_eq!(c.finish(), Err(Error::InvalidTransition));
}
#[test]
fn manual_selection_accepts_only_tied_leaders() {
    let candidates = vec![
        Candidate {
            key: "b".into(),
            weight: 10,
        },
        Candidate {
            key: "a".into(),
            weight: 10,
        },
        Candidate {
            key: "c".into(),
            weight: 1,
        },
    ];
    assert_eq!(
        select_manual(candidates.clone(), "c"),
        Err(Error::InvalidCandidate)
    );
    assert_eq!(
        select_manual(candidates.clone(), "missing"),
        Err(Error::InvalidCandidate)
    );
    let selected = select_manual(candidates, "b").unwrap();
    assert_eq!(selected.winner.as_deref(), Some("b"));
    assert_eq!(selected.eligible[0].key, "a");
    selected.validate().unwrap();
    assert_eq!(
        select_manual(
            vec![Candidate {
                key: "a".into(),
                weight: 1
            }],
            "a"
        ),
        Err(Error::InvalidCandidate)
    );
}
#[test]
fn weighted_draw_distribution_and_overflow() {
    let candidates = vec![
        Candidate {
            key: "a".into(),
            weight: 1,
        },
        Candidate {
            key: "b".into(),
            weight: 3,
        },
    ];
    let mut a = 0;
    for _ in 0..4000 {
        if select(
            candidates.clone(),
            SelectionMode::WeightedRandom,
            TiePolicy::Stable,
        )
        .unwrap()
        .winner
        .as_deref()
            == Some("a")
        {
            a += 1;
        }
    }
    // A broad interval catches gross bias without making sampling noise flaky.
    assert!((800..=1200).contains(&a), "observed {a} of 4000");
    assert_eq!(
        select(
            vec![
                Candidate {
                    key: "a".into(),
                    weight: u64::MAX
                },
                Candidate {
                    key: "b".into(),
                    weight: 1
                }
            ],
            SelectionMode::WeightedRandom,
            TiePolicy::Stable
        ),
        Err(Error::Overflow)
    );
}
#[test]
fn selection_boundaries_and_ties() {
    let c = vec![
        Candidate {
            key: "b".into(),
            weight: 0,
        },
        Candidate {
            key: "a".into(),
            weight: 4,
        },
    ];
    for _ in 0..100 {
        assert_eq!(
            select(c.clone(), SelectionMode::WeightedRandom, TiePolicy::Stable)
                .unwrap()
                .winner
                .as_deref(),
            Some("a")
        );
    }
    let tied = vec![
        Candidate {
            key: "b".into(),
            weight: 1,
        },
        Candidate {
            key: "a".into(),
            weight: 1,
        },
    ];
    assert_eq!(
        select(tied.clone(), SelectionMode::HighestScore, TiePolicy::Stable)
            .unwrap()
            .winner
            .as_deref(),
        Some("a")
    );
    assert_eq!(
        select(tied, SelectionMode::HighestScore, TiePolicy::Manual),
        Err(Error::ManualTie)
    );
    assert_eq!(
        select(
            vec![Candidate {
                key: "a".into(),
                weight: 0
            }],
            SelectionMode::WeightedRandom,
            TiePolicy::Stable
        ),
        Err(Error::ZeroWeight)
    );
}
