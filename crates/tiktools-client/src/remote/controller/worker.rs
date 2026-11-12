//! Recovery stream processing, backoff and permission-aware reconnection.
use super::*;

pub(super) async fn run(
    shared: Arc<Shared>,
    commands: Arc<Mutex<()>>,
    epoch: String,
    mut remote: RemoteClient,
    mut recovery: Option<RecoveryStream>,
) {
    let mut backoff = ReconnectBackoff::default();
    let mut clock = tokio::time::interval_at(
        tokio::time::Instant::now() + points::period(&remote),
        points::period(&remote),
    );
    clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        let result = if let Some(stream) = &mut recovery {
            tokio::select! {
                event=stream.next_event()=>match event {
                    Ok(event) if event.resource.kind==ResourceKind::Viewer && event.topic=="points.changed"=>{points::dirty(&shared,&epoch);stream.acknowledge(&event)},
                    Ok(event) if event.resource.kind==ResourceKind::Competition && event.topic=="competition.changed"=>match competitions::changed(&shared,&epoch,&remote,&event) {Ok(())=>stream.acknowledge(&event),Err(e)=>Err(e)},
                    Ok(event) if event.resource.kind==ResourceKind::Automation=>match automations::apply(&shared,&epoch,&remote,&event).await {Ok(())=>stream.acknowledge(&event),Err(e)=>Err(e)},
                    Ok(event)=>match apply(&shared,&epoch,&remote,&event).await {Ok(())=>stream.acknowledge(&event),Err(e)=>Err(e)},
                    Err(e)=>Err(e),
                },
                _=clock.tick(), if points::readable(&remote)=>{
                    if points::due(&shared,&epoch) {
                        match points::refresh(&shared,&epoch,&remote).await {
                            Ok(())=>Ok(()),
                            Err(e) if matches!(e.code.as_str(),"too_large"|"internal"|"storage"|"rate_limited")=>{points::failure(&shared,&epoch,&e);Ok(())},
                            Err(e)=>Err(e),
                        }
                    } else {Ok(())}
                }
            }
        } else {
            // Without a domain stream there is no epoch-gap notification.
            // Refresh hello when the live identity's permissions change.
            // A minimal-rate host needs room for the controller's permitted
            // reads as well as this authorization heartbeat. A rejected
            // heartbeat is retried; it is not evidence of a lost connection.
            let period = if remote
                .hello()
                .capabilities
                .contains(&Capability::CompetitionsRead)
                && remote.hello().limits.requests_per_second <= 2
            {
                5
            } else {
                1
            };
            tokio::time::sleep(Duration::from_secs(period)).await;
            if remote.is_closed() {
                Err(network())
            } else {
                match remote
                    .call_value_with_meta("collaboration.me", json!({}), None)
                    .await
                {
                    Ok(me)
                        if me.result.get("grantRevision").and_then(Value::as_str)
                            == Some(remote.hello().grant_revision.as_str()) =>
                    {
                        continue
                    }
                    Ok(_) => Err(fail("resync_required", "remote permissions changed")),
                    Err(e) if e.code == "rate_limited" => continue,
                    Err(e) => Err(e),
                }
            }
        };
        if result.is_ok() {
            continue;
        }
        let e = result.unwrap_err();
        if !matches!(
            e.code.as_str(),
            "transport"
                | "forbidden"
                | "resync_required"
                | "unauthorized"
                | "overloaded"
                | "rate_limited"
                | "timeout"
        ) {
            remote.close();
            shared.change(&epoch, |v| {
                v.rows.clear();
                automations::clear(v);
                v.remote = None;
            });
            shared.phase(&epoch, "failed", Some(&e));
            return;
        }
        shared.change(&epoch, points::clear);
        shared.phase(&epoch, "reconnecting", Some(&e));
        loop {
            tokio::time::sleep(backoff.next_delay()).await;
            let mut reconnected = if let Some(stream) = &mut recovery {
                stream.reconnect().await.map(|mode| {
                    (
                        stream.connection().clone(),
                        if mode == RecoveryMode::Snapshot {
                            stream.baseline().cloned()
                        } else {
                            None
                        },
                    )
                })
            } else {
                remote.close();
                remote.reopen().await.map(|r| (r, None))
            };
            // A new grant can remove the old stream's domains. Negotiate the
            // same profile again, then construct only its current domains.
            // No business command is retried in this recovery path.
            let changed_domains = reconnected.as_ref().is_ok_and(|(r, _)| {
                recovery
                    .as_ref()
                    .is_some_and(|s| s.domains() != &points::domains(r))
            });
            let rebuild = changed_domains
                || reconnected
                    .as_ref()
                    .is_err_and(|e| matches!(e.code.as_str(), "forbidden" | "unsupported_remote"));
            if rebuild {
                if let Ok((r, _)) = &reconnected {
                    r.close();
                }
                reconnected = remote.reopen().await.map(|r| (r, None));
            }
            if let Ok((r, baseline)) = &mut reconnected {
                if rebuild || recovery.is_none() {
                    shared.change(&epoch, |v| {
                        v.rows.clear();
                        automations::clear(v);
                    });
                    recovery = None;
                    match points::stream(r).await {
                        Ok(stream) => {
                            *baseline = stream.as_ref().and_then(|s| s.baseline()).cloned();
                            recovery = stream;
                        }
                        Err(e) => reconnected = Err(e),
                    }
                }
            }
            match reconnected {
                Ok((r, baseline)) => {
                    remote = r;
                    shared.change(&epoch, |v| v.remote = Some(remote.clone()));
                    points::reset_scope(&shared, &epoch, &remote);
                    if let Err(e) = points::refresh(&shared, &epoch, &remote).await {
                        if matches!(
                            e.code.as_str(),
                            "forbidden" | "unauthorized" | "resync_required" | "transport"
                        ) {
                            shared.phase(&epoch, "reconnecting", Some(&e));
                            continue;
                        }
                        points::failure(&shared, &epoch, &e);
                    }
                    if let Err(e) = shared.ready(&epoch, &remote, baseline.as_ref()) {
                        remote.close();
                        shared.change(&epoch, |v| {
                            automations::clear(v);
                            v.rows.clear();
                            v.remote = None;
                        });
                        shared.phase(&epoch, "failed", Some(&e));
                        return;
                    }
                    clock = tokio::time::interval_at(
                        tokio::time::Instant::now() + points::period(&remote),
                        points::period(&remote),
                    );
                    clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                    commands::schedule_resolve(shared.clone(), commands.clone(), epoch.clone());
                    backoff.reset();
                    break;
                }
                Err(e)
                    if matches!(
                        e.code.as_str(),
                        "transport" | "profile_in_use" | "overloaded" | "rate_limited" | "timeout"
                    ) =>
                {
                    shared.phase(&epoch, "reconnecting", Some(&e));
                }
                Err(e) => {
                    remote.close();
                    shared.change(&epoch, |v| {
                        v.rows.clear();
                        automations::clear(v);
                        v.remote = None;
                    });
                    shared.phase(&epoch, "failed", Some(&e));
                    return;
                }
            }
        }
    }
}
