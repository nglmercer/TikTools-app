//! Durable competition outbox dispatch through the existing automation runtime.
use crate::{recover_mutex, AppCore};
use std::{
    sync::{atomic::Ordering, Arc},
    time::Duration,
};
use tiktools_competitions::ActionClaim;

impl AppCore {
    pub(crate) fn start_competition_action_worker(self: &Arc<Self>) {
        let mut slot = recover_mutex(
            &self.competitions.action_worker,
            "competition action worker",
        );
        if slot.is_some() {
            return;
        }
        let weak = Arc::downgrade(self);
        *slot = Some(tokio::spawn(async move {
            // Nothing has been claimed by this process yet, so any `running`
            // row belongs to a host that died mid-action. Its side effect may
            // or may not have happened: mark it uncertain for explicit
            // resolution, never re-run it automatically.
            if let Some(core) = weak.upgrade() {
                let db = core.db.clone();
                let now = crate::now_millis().min(i64::MAX as u64) as i64;
                match tokio::task::spawn_blocking(move || {
                    db.mark_interrupted_competition_actions_uncertain(now)
                })
                .await
                {
                    Ok(Ok(0)) => {}
                    Ok(Ok(count)) => tracing::warn!(
                        count,
                        "interrupted competition actions marked uncertain; resolve them explicitly"
                    ),
                    Ok(Err(error)) => {
                        tracing::error!(%error, "cannot recover interrupted competition actions")
                    }
                    Err(error) => tracing::error!(%error, "action recovery worker failed"),
                }
            }
            loop {
                let Some(core) = weak.upgrade() else {
                    break;
                };
                if core.shutdown_started.load(Ordering::Acquire) {
                    break;
                }
                let db = core.db.clone();
                let pending =
                    tokio::task::spawn_blocking(move || db.pending_competition_actions()).await;
                match pending {
                    Ok(Ok(items)) => {
                        for (id, generation) in items {
                            if core.shutdown_started.load(Ordering::Acquire) {
                                break;
                            }
                            *recover_mutex(
                                &core.competitions.action_inflight,
                                "competition action in flight",
                            ) = Some((id.clone(), generation));
                            let db = core.db.clone();
                            let claim_id = id.clone();
                            let claimed = tokio::task::spawn_blocking(move || {
                                db.claim_competition_action_detailed(
                                    &claim_id,
                                    generation,
                                    crate::now_millis().min(i64::MAX as u64) as i64,
                                )
                            })
                            .await;
                            match claimed {
                                Ok(Ok(crate::db::ActionClaimOutcome::Claimed(claim))) => {
                                    core.execute_competition_action(claim).await
                                }
                                Ok(Ok(crate::db::ActionClaimOutcome::Missing)) => {}
                                Ok(Ok(crate::db::ActionClaimOutcome::Invalid { action_id })) => {
                                    if let Some(mut data) =
                                        core.competition_context(&id, generation)
                                    {
                                        data.action_id = Some(action_id);
                                        data.action_status =
                                            Some(tiktools_competitions::ActionStatus::Failed);
                                        core.publish_competition_event(
                                            "competition.action.failed",
                                            data,
                                        );
                                    }
                                }
                                Ok(Err(error)) => {
                                    tracing::error!(%error, "competition action claim failed")
                                }
                                Err(error) => {
                                    tracing::error!(%error, "competition action claim worker failed")
                                }
                            }
                            *recover_mutex(
                                &core.competitions.action_inflight,
                                "competition action in flight",
                            ) = None;
                        }
                    }
                    Ok(Err(error)) => tracing::error!(%error, "competition outbox read failed"),
                    Err(error) => tracing::error!(%error, "competition outbox worker failed"),
                }
                drop(core);
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        }));
    }

    async fn execute_competition_action(self: &Arc<Self>, claim: ActionClaim) {
        let core = Arc::clone(self);
        let lookup = claim.clone();
        let prepared = tokio::task::spawn_blocking(move || -> Result<_, &'static str> {
            let result = core.db.load_competition_result(&lookup.competition_id, lookup.generation).map_err(|_| "result_unavailable")?.ok_or("result_unavailable")?;
            let definition = core.competition_get(&lookup.competition_id).map_err(|_| "competition_unavailable")?;
            let winner = result.selection.winner.clone().ok_or("winner_unavailable")?;
            if definition.options.iter().find(|option| option.id == winner).and_then(|option| option.on_win.as_ref()) != Some(&lookup.action) { return Err("action_reference_mismatch"); }
            let ranking = core.ranking_get(&definition.ranking_id).map_err(|_| "ranking_unavailable")?;
            let mut action = core.automation_get(crate::control::AutomationKind::Action, &lookup.action.action_id).map_err(|_| "action_unavailable")?;
            if action.get("enabled").and_then(serde_json::Value::as_bool) == Some(false) { return Err("action_disabled"); }
            let inputs = match &lookup.action.inputs {
                serde_json::Value::Null => serde_json::Map::new(),
                serde_json::Value::Object(inputs) => inputs.clone(),
                _ => return Err("invalid_inputs"),
            };
            let object = action.as_object_mut().ok_or("invalid_action")?;
            let config = object.entry("config").or_insert_with(|| serde_json::json!({})).as_object_mut().ok_or("invalid_action")?;
            config.extend(inputs);
            let score = result.selection.eligible.iter().find(|candidate| candidate.key == winner).ok_or("winner_unavailable")?.weight;
            let event = serde_json::json!({"id": lookup.idempotency_key, "type": "competition.winner.selected", "timestamp": result.finalized_at,
                "creator": {"uniqueId": ranking.creator_id, "roomId": ranking.room_id},
                "data": {"competitionId": lookup.competition_id, "generation": lookup.generation,
                    "optionId": winner, "winnerId": winner, "winnerScore": score,
                    "rankingId": result.ranking_id, "selectionMode": result.selection.mode,
                    "selectionId": lookup.idempotency_key, "actionId": lookup.action.action_id} });
            Ok((action, event))
        }).await;
        let competition_id = claim.competition_id.clone();
        let generation_id = claim.generation;
        let action_id = claim.action.action_id.clone();
        let (action, event) = match prepared {
            Ok(Ok(value)) => value,
            error => {
                let reason = match error {
                    Ok(Err(reason)) => reason,
                    Err(_) => "preparation_interrupted",
                    Ok(Ok(_)) => unreachable!(),
                };
                let db = self.db.clone();
                let id = claim.competition_id;
                let generation = claim.generation;
                let outcome = tokio::task::spawn_blocking(move || {
                    db.record_competition_action_outcome(
                        &id,
                        generation,
                        tiktools_competitions::ActionStatus::Failed,
                        Some(reason),
                        crate::now_millis().min(i64::MAX as u64) as i64,
                    )
                })
                .await;
                if matches!(outcome, Ok(Ok(true))) {
                    if let Some(mut data) = self.competition_context(&competition_id, generation_id)
                    {
                        data.action_id = Some(action_id);
                        data.action_status = Some(tiktools_competitions::ActionStatus::Failed);
                        self.publish_competition_event("competition.action.failed", data);
                    }
                } else {
                    tracing::error!(
                        "competition action preparation failure could not be persisted"
                    );
                }
                return;
            }
        };
        let interrupted = async {
            while !self.shutdown_started.load(Ordering::Acquire) {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        };
        let run = tokio::select! {
            biased;
            _ = interrupted => None,
            run = self.execute_action(&action, &event, None, false) => Some(run),
        };
        let interrupted = run.is_none();
        let succeeded = run.as_ref().is_some_and(|value| {
            value.get("status").and_then(serde_json::Value::as_str) == Some("ok")
        });
        let db = self.db.clone();
        let id = claim.competition_id;
        let generation = claim.generation;
        let persisted = tokio::task::spawn_blocking(move || {
            let now = crate::now_millis().min(i64::MAX as u64) as i64;
            let status = if succeeded {
                tiktools_competitions::ActionStatus::Completed
            } else {
                tiktools_competitions::ActionStatus::Uncertain
            };
            let reason = if succeeded {
                None
            } else if interrupted {
                Some("execution_interrupted")
            } else {
                Some("execution_failed")
            };
            db.record_competition_action_outcome(&id, generation, status, reason, now)
        })
        .await;
        if !matches!(persisted, Ok(Ok(true))) {
            tracing::error!(
                "competition action outcome could not be persisted; inspect running state"
            );
        } else if let Some(mut data) = self.competition_context(&competition_id, generation_id) {
            data.action_id = Some(action_id);
            data.action_status = Some(if succeeded {
                tiktools_competitions::ActionStatus::Completed
            } else {
                tiktools_competitions::ActionStatus::Uncertain
            });
            data.option_id = event
                .pointer("/data/optionId")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            data.winner_id = event
                .pointer("/data/winnerId")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            data.winner_score = event
                .pointer("/data/winnerScore")
                .and_then(serde_json::Value::as_u64);
            data.selection_mode =
                serde_json::from_value(event["data"]["selectionMode"].clone()).ok();
            data.selection_id = event
                .pointer("/data/selectionId")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            self.publish_competition_event(
                if succeeded {
                    "competition.action.completed"
                } else {
                    "competition.action.failed"
                },
                data,
            );
        }
    }

    pub(crate) async fn stop_competition_action_worker(&self) {
        let task = recover_mutex(
            &self.competitions.action_worker,
            "competition action worker",
        )
        .take();
        if let Some(mut task) = task {
            if tokio::time::timeout(Duration::from_secs(5), &mut task)
                .await
                .is_err()
            {
                task.abort();
                let _ = task.await;
                tracing::error!("competition action worker stopped with an unknown outcome; inspect running actions");
            }
        }
        *recover_mutex(
            &self.competitions.action_inflight,
            "competition action in flight",
        ) = None;
    }
}
