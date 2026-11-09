//! Opt-in deterministic synthetic input through the real bounded ingress worker.
//! This does not connect to TikTok. Never infer real-LIVE results from it.
use crate::live_events::ingress::{
    admit_stamped_event, run_ingress_worker, INGRESS_QUEUE_CAPACITY,
};
use crate::*;
use tiktools_tiktok::events::{
    CanonicalLiveEvent, ChatEvent, EventMetadata, EventUser, GiftEnrichment, GiftEvent, LikeEvent,
};

pub struct LiveBenchmarkInput {
    core: Arc<AppCore>,
    sender: tokio::sync::mpsc::Sender<crate::live_events::ingress::IngressItem>,
    pub submitted: u64,
}

impl AppCore {
    /// Isolated benchmark hosts only. Starting a second producer is rejected;
    /// shutdown owns and joins the worker through the normal shutdown ladder.
    pub async fn start_live_benchmark(
        self: &Arc<Self>,
    ) -> Result<LiveBenchmarkInput, &'static str> {
        if self.live_pump_started.swap(true, Ordering::AcqRel) || self.is_shutdown() {
            return Err("LIVE pipeline already started or shutdown");
        }
        let (sender, receiver) = tokio::sync::mpsc::channel(INGRESS_QUEUE_CAPACITY);
        let core = self.clone();
        let stop = self.live_ingress_stop.clone();
        *recover_mutex(&self.live_ingress_task, "benchmark ingress") =
            Some(tokio::spawn(run_ingress_worker(core, receiver, stop)));
        admit_stamped_event(
            self,
            &sender,
            tiktools_tiktok::StampedClientEvent {
                generation: self.connection_sequence.load(Ordering::Acquire),
                event: ClientEvent::Connected(tiktools_tiktok::ConnectionInfo {
                    unique_id: "synthetic-benchmark".to_owned(),
                    verified_creator: Some("synthetic-benchmark".to_owned()),
                    room_id: "benchmark".to_owned(),
                    title: "Synthetic benchmark".to_owned(),
                    nickname: "Benchmark".to_owned(),
                    avatar_url: None,
                    viewers: 100,
                    total_users: 100,
                    gifts: vec![],
                }),
            },
        )
        .await;
        Ok(LiveBenchmarkInput {
            core: self.clone(),
            sender,
            submitted: 0,
        })
    }
}

impl LiveBenchmarkInput {
    /// Same index always produces the same payload. Use a strictly increasing
    /// index so replay protection does not mistake benchmark events for replays.
    /// URLs are supplied by the harness, keeping workloads independent of CDNs.
    pub async fn submit(&mut self, index: u64, viewers: u64, mixed: bool, image_url: Option<&str>) {
        let user = EventUser {
            id: index % viewers.max(1) + 1,
            unique_id: format!("bench-{}", index % viewers.max(1)),
            nickname: "Benchmark viewer".to_owned(),
            sec_uid: String::new(),
            avatar_url: image_url.map(str::to_owned),
        };
        let (base, gift) = if mixed && index.is_multiple_of(10) {
            (
                CanonicalLiveEvent::Gift(GiftEvent {
                    user,
                    gift_id: 1,
                    gift_name: "Benchmark gift".to_owned(),
                    diamond_count: 1,
                    repeat_count: 1,
                    combo_count: 1,
                    group_id: index + 1,
                    repeat_end: true,
                    ..Default::default()
                }),
                Some(GiftEnrichment {
                    name: "Benchmark gift".to_owned(),
                    diamond_count: 1,
                    streakable: false,
                    icon_url: image_url.map(str::to_owned),
                }),
            )
        } else if mixed && index % 10 == 1 {
            (
                CanonicalLiveEvent::Like(LikeEvent {
                    user,
                    count: 10,
                    total: index * 10,
                }),
                None,
            )
        } else {
            (
                CanonicalLiveEvent::Chat(ChatEvent {
                    user,
                    comment: format!("benchmark message {index}"),
                    ..Default::default()
                }),
                None,
            )
        };
        admit_stamped_event(
            &self.core,
            &self.sender,
            tiktools_tiktok::StampedClientEvent {
                generation: self.core.connection_sequence.load(Ordering::Acquire),
                event: ClientEvent::Event(NativeLiveEvent {
                    base,
                    gift,
                    metadata: EventMetadata {
                        method: "WebcastBenchmark".to_owned(),
                        msg_id: index + 1,
                        is_history: false,
                    },
                }),
            },
        )
        .await;
        self.submitted += 1;
    }
}
