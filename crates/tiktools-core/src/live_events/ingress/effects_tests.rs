
use super::tests::isolated_core;
use super::*;
use tiktools_tiktok::events::{CanonicalLiveEvent, ChatEvent, EventMetadata, EventUser};
struct Emitter;
impl HostEmitter for Emitter {
    fn emit(&self, _: HostMessage) {}
}
#[tokio::test]
async fn stalled_persistence_never_hides_originals_and_saturation_is_accounted() {
    let core = isolated_core(Arc::new(Emitter));
    core.live_points_work
        .stall_for_test(Duration::from_millis(200));
    let (sender, receiver) = tokio::sync::mpsc::channel(INGRESS_QUEUE_CAPACITY);
    for id in 1..=300 {
        admit_stamped_event(
            &core,
            &sender,
            tiktools_tiktok::StampedClientEvent {
                generation: 0,
                event: ClientEvent::Event(NativeLiveEvent {
                    base: CanonicalLiveEvent::Chat(ChatEvent {
                        user: EventUser {
                            id: 1,
                            unique_id: "effects-test".into(),
                            ..Default::default()
                        },
                        comment: "same".into(),
                        ..Default::default()
                    }),
                    metadata: EventMetadata {
                        method: "WebcastChatMessage".into(),
                        msg_id: id,
                        is_history: false,
                    },
                    gift: None,
                }),
            },
        )
        .await;
    }
    drop(sender);
    let worker = tokio::spawn(run_ingress_worker(
        Arc::clone(&core),
        receiver,
        tokio_util::sync::CancellationToken::new(),
    ));
    tokio::time::timeout(Duration::from_secs(5), async {
        while core.live_ingress.ui_published.load(Ordering::Relaxed) < 300 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(core.live_ingress.effects_dropped.load(Ordering::Relaxed) > 0);
    core.live_points_work.stall_for_test(Duration::ZERO);
    tokio::time::timeout(Duration::from_secs(5), worker)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        core.live_ingress.effects_processed.load(Ordering::Relaxed)
            + core.live_ingress.effects_dropped.load(Ordering::Relaxed),
        300
    );
    assert_eq!(core.live_ingress.effects_queued.load(Ordering::Relaxed), 0);
    assert_eq!(
        core.live_ingress.effects_in_flight.load(Ordering::Relaxed),
        0
    );
    core.shutdown().await;
}
