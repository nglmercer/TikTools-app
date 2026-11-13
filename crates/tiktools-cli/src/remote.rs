//! Native profile management never resolves local IPC or constructs AppCore.
#[cfg(feature = "cloud-auth")]
mod cloud;
use crate::client::{exit_code_for, print_error};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    io::{self, BufRead, Read},
    time::Duration,
};
use tiktools_client::{
    remote::{
        normalize_fingerprint, parse_invitation, probe_fingerprint, profiles::ProfileManager,
        ClientKind, PairingPoll, RemoteEndpoint, TlsTrust,
    },
    ClientError,
};
use zeroize::Zeroizing;

pub async fn run(args: &[String], json_mode: bool) -> i32 {
    #[cfg(feature = "cloud-auth")]
    if args.first().is_some_and(|arg| arg == "cloud") {
        return cloud::run(&args[1..], json_mode).await;
    }
    let plan = match parse(args) {
        Ok(p) => p,
        Err(message) => {
            eprintln!("tiktools: {message}");
            return 2;
        }
    };
    match execute(plan).await {
        Ok(value) => {
            if json_mode {
                println!("{value}");
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&value).unwrap_or_default()
                );
            }
            0
        }
        Err(error) => {
            print_error(json_mode, &error);
            exit_code_for(&error)
        }
    }
}
#[derive(Debug, PartialEq)]
enum Plan {
    List,
    Show(String),
    Check(String),
    Forget(String),
    Inspect(String),
    Trust {
        name: String,
        pin: String,
    },
    Pair {
        url: String,
        name: String,
        pin: Option<String>,
        stdin: bool,
        label: String,
        kind: String,
    },
}
fn options(
    args: &[String],
    allowed: &[&str],
    flags: &[&str],
) -> Result<BTreeMap<String, String>, String> {
    let mut result = BTreeMap::new();
    let mut args = args.iter();
    while let Some(key) = args.next() {
        if !allowed.contains(&key.as_str()) && !flags.contains(&key.as_str()) {
            return Err("unsupported remote command option".into());
        }
        if result.contains_key(key) {
            return Err("duplicate remote command option".into());
        }
        let value = if flags.contains(&key.as_str()) {
            String::new()
        } else {
            args.next()
                .filter(|s| !s.starts_with("--"))
                .ok_or("remote option requires a value")?
                .to_string()
        };
        result.insert(key.clone(), value);
    }
    Ok(result)
}
fn parse(args: &[String]) -> Result<Plan, String> {
    let name = |s: &str| {
        if tiktools_client::remote::profiles::valid_name(s) {
            Ok(s.to_owned())
        } else {
            Err("invalid remote profile name".to_owned())
        }
    };
    match args.first().map(String::as_str){
        Some("list") if args.len()==1=>Ok(Plan::List),
        Some("check") if args.len()==2=>Ok(Plan::Check(name(&args[1])?)),
        Some("show")|Some("forget") if args.len()==2=>if args[0]=="show"{Ok(Plan::Show(name(&args[1])?))}else{Ok(Plan::Forget(name(&args[1])?))},
        Some("inspect")=>{let mut o=options(&args[1..],&["--url"],&[])?;Ok(Plan::Inspect(o.remove("--url").ok_or("--url is required")?))},
        Some("trust") if args.len()>=2=>{let o=options(&args[2..],&["--fingerprint"],&["--confirm"])?;if !o.contains_key("--confirm"){return Err("trust changes require --confirm after comparing the fingerprint with the owner".into());}let pin=o.get("--fingerprint").ok_or("--fingerprint is required")?;let pin=normalize_fingerprint(pin).map_err(|_|"invalid certificate fingerprint")?;Ok(Plan::Trust{name:name(&args[1])?,pin})},
        Some("pair")=>{let mut o=options(&args[1..],&["--url","--profile","--fingerprint","--label","--kind"],&["--invite-stdin"])?;let url=o.remove("--url").ok_or("--url is required")?;let profile=name(&o.remove("--profile").ok_or("--profile is required")?)?;let pin=o.remove("--fingerprint").map(|p|normalize_fingerprint(&p).map_err(|_|"invalid certificate fingerprint")).transpose()?;let kind=o.remove("--kind").unwrap_or_else(||"agent".into());if !matches!(kind.as_str(),"agent"|"desktop"){return Err("--kind must be agent or desktop".into());}Ok(Plan::Pair{url,name:profile,pin,stdin:o.contains_key("--invite-stdin"),label:o.remove("--label").unwrap_or_else(||"TikTools CLI".into()),kind})},
        _=>Err("use remote pair|inspect|list|show|check|forget|trust; invitation secrets are entered privately, never as command arguments".into())
    }
}
async fn execute(plan: Plan) -> Result<Value, ClientError> {
    if let Plan::Inspect(url) = plan {
        let pin = probe_fingerprint(&url).await?;
        return Ok(
            json!({"certificateFingerprint":pin,"verified":false,"note":"Compare this unverified fingerprint with the owner before selecting it."}),
        );
    }
    let manager = ProfileManager::system()?;
    match plan {
        Plan::List => Ok(json!({"profiles":manager.list().await?})),
        Plan::Show(name) => Ok(serde_json::to_value(manager.get(&name).await?)
            .map_err(|_| error("protocol", "invalid profile shape"))?),
        Plan::Check(name) => {
            let started = tokio::time::Instant::now();
            let remote = manager.open(&name).await?;
            remote
                .call_value_with_meta("collaboration.me", json!({}), None)
                .await?;
            let route_started = tokio::time::Instant::now();
            let route = loop {
                if remote.is_closed() {
                    return Err(error(
                        "transport",
                        "remote connection closed during verification",
                    ));
                }
                if remote.peer_endpoint_id().is_none() {
                    break "https";
                }
                match remote.peer_route() {
                    Some(tiktools_client::remote::PeerRoute::Direct) => break "direct",
                    Some(tiktools_client::remote::PeerRoute::Relay) => break "relay",
                    Some(tiktools_client::remote::PeerRoute::Reconnecting) | None => {}
                }
                if route_started.elapsed() >= Duration::from_secs(2) {
                    return Err(error(
                        "unavailable",
                        "native transport has not reported its selected route",
                    ));
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            };
            let hello = remote.hello();
            // Deliberately omit credentials, endpoint addresses and policy data.
            let report = json!({"connected":true,"route":route,"connectMs":started.elapsed().as_millis(),
                "hostId":hello.host_id,"hostInstanceId":hello.host_instance_id,"connectionId":hello.connection_id,
                "principalId":hello.principal_id,"sessionId":hello.session_id,"grantRevision":hello.grant_revision});
            remote.close();
            Ok(report)
        }
        Plan::Forget(name) => {
            manager.forget(&name).await?;
            Ok(json!({"forgotten":true,"hostSessionRevoked":false}))
        }
        Plan::Trust { name, pin } => {
            eprintln!("Approved certificate SHA-256: {pin}");
            Ok(serde_json::to_value(
                manager
                    .set_trust(&name, TlsTrust::CertificateSha256 { sha256: pin })
                    .await?,
            )
            .map_err(|_| error("protocol", "invalid profile shape"))?)
        }
        Plan::Pair {
            url,
            name,
            pin,
            stdin,
            label,
            kind,
        } => {
            manager.ensure_new(&name).await?;
            manager.check_store().await?;
            let trust = if let Some(pin) = pin {
                eprintln!("Approved certificate SHA-256: {pin}");
                TlsTrust::CertificateSha256 { sha256: pin }
            } else {
                TlsTrust::System
            };
            let endpoint = RemoteEndpoint::new(&url, trust).await?;
            let info = endpoint.info().await?;
            eprintln!("Pairing with {} (host {})", endpoint.origin(), info.host_id);
            let input = tokio::task::spawn_blocking(move || private_invitation(stdin))
                .await
                .map_err(|_| error("input", "private invitation input failed"))??;
            let (invite_id, secret) = parse_invitation(&input, endpoint.origin())?;
            let kind = if kind == "agent" {
                ClientKind::Agent
            } else {
                ClientKind::Desktop
            };
            let mut pending = endpoint
                .begin_pairing(&invite_id, &secret, &label, kind)
                .await?;
            eprintln!(
                "Waiting for owner approval of request {}",
                pending.request_id
            );
            loop {
                match pending.poll().await? {
                    PairingPoll::Pending => tokio::time::sleep(Duration::from_millis(1500)).await,
                    PairingPoll::Approved(credentials) => {
                        return serde_json::to_value(
                            manager.install(&name, &endpoint, credentials, kind).await?,
                        )
                        .map_err(|_| error("protocol", "invalid profile shape"))
                    }
                }
            }
        }
        Plan::Inspect(_) => unreachable!(),
    }
}
fn error(code: &str, message: &str) -> ClientError {
    ClientError::new(code, message)
}
fn private_invitation(stdin: bool) -> Result<Zeroizing<String>, ClientError> {
    let value = if stdin {
        let mut value = String::new();
        io::stdin()
            .lock()
            .take(8193)
            .read_line(&mut value)
            .map_err(|_| error("input", "could not read private invitation input"))?;
        value
    } else {
        rpassword::prompt_password("Invitation JSON or join link (hidden): ")
            .map_err(|_| error("input", "could not read private invitation input"))?
    };
    let value = Zeroizing::new(value);
    if value.len() > 8192 || value.trim().is_empty() {
        return Err(error("invalid_params", "invalid private invitation input"));
    }
    Ok(value)
}

pub async fn events(
    remote: tiktools_client::remote::RemoteClient,
    topics: &[String],
    timeout_secs: Option<u64>,
    max_events: Option<u64>,
) -> Result<(), ClientError> {
    use tiktools_client::remote::{ReconnectBackoff, RecoveryStream};
    let allowed = |topic: &str| {
        topics.is_empty()
            || topics.iter().any(|p| {
                p == "*"
                    || p == topic
                    || p.strip_suffix('*')
                        .is_some_and(|prefix| topic.starts_with(prefix))
            })
    };
    let hello = remote.hello();
    let grant = tiktools_core::collaboration::Grant {
        capabilities: hello.capabilities.clone(),
        resources: hello.resource_restrictions.clone(),
        ..Default::default()
    };
    let domains = event_domains(&grant, &hello.subscription_domains, allowed);
    if domains.is_empty() {
        return Err(error(
            "unsupported_remote",
            "no requested remote event domain is currently permitted and supported",
        ));
    }
    let deadline = timeout_secs.map(|secs| tokio::time::Instant::now() + Duration::from_secs(secs));
    let mut recovery = if let Some(deadline) = deadline {
        tokio::time::timeout_at(deadline, RecoveryStream::start(remote, domains))
            .await
            .map_err(|_| {
                error(
                    "timeout",
                    "remote baseline did not finish before the event deadline",
                )
            })??
    } else {
        RecoveryStream::start(remote, domains).await?
    };
    let mut count = 0;
    let mut backoff = ReconnectBackoff::default();
    eprintln!("tiktools: listening to the selected remote profile (Ctrl-C to stop)");
    loop {
        if max_events.is_some_and(|max| count >= max) {
            return Ok(());
        }
        let event = if let Some(deadline) = deadline {
            match tokio::time::timeout_at(deadline, recovery.next_event()).await {
                Ok(event) => event,
                Err(_) => return Ok(()),
            }
        } else {
            recovery.next_event().await
        };
        match event {
            Ok(event) => {
                if allowed(&event.topic) {
                    println!(
                        "{}",
                        serde_json::to_string(&event)
                            .map_err(|_| error("protocol", "invalid reliable event"))?
                    );
                    count += 1;
                }
                recovery.acknowledge(&event)?;
                backoff.reset();
            }
            Err(error)
                if matches!(
                    error.code.as_str(),
                    "transport" | "resync_required" | "unauthorized"
                ) =>
            {
                println!("{}", json!({"topic":"event.gap","data":{"resync":true}}));
                count += 1;
                if max_events.is_some_and(|max| count >= max) {
                    return Ok(());
                }
                loop {
                    let reconnect = async {
                        tokio::time::sleep(backoff.next_delay()).await;
                        recovery.reconnect().await
                    };
                    let result = if let Some(deadline) = deadline {
                        match tokio::time::timeout_at(deadline, reconnect).await {
                            Ok(result) => result,
                            Err(_) => return Ok(()),
                        }
                    } else {
                        reconnect.await
                    };
                    match result {
                        Ok(_) => {
                            backoff.reset();
                            break;
                        }
                        Err(error)
                            if matches!(
                                error.code.as_str(),
                                "transport" | "profile_in_use" | "overloaded" | "timeout"
                            ) => {}
                        Err(error) => return Err(error),
                    }
                }
            }
            Err(error) => return Err(error),
        }
    }
}

fn event_domains(
    grant: &tiktools_core::collaboration::Grant,
    supported: &[tiktools_core::collaboration::StreamDomain],
    allowed: impl Fn(&str) -> bool,
) -> std::collections::BTreeSet<tiktools_core::collaboration::StreamDomain> {
    use tiktools_core::collaboration::{Capability, ResourceKind, StreamDomain};
    let workflows = grant.capabilities.contains(&Capability::WorkflowsRead)
        && supported.contains(&StreamDomain::Workflows);
    let points = grant.capabilities.contains(&Capability::PointsRead)
        && supported.contains(&StreamDomain::Points)
        && !grant.resources.contains_key(&ResourceKind::Live);
    let mut domains = std::collections::BTreeSet::new();
    for (domain, topic, permitted) in [
        (StreamDomain::Workflows, "workflow.changed", workflows),
        (StreamDomain::Points, "points.changed", points),
        (
            StreamDomain::Commands,
            "command.changed",
            (workflows || points) && grant.capabilities.contains(&Capability::CommandsRead),
        ),
    ] {
        if permitted && supported.contains(&domain) && allowed(topic) {
            domains.insert(domain);
        }
    }
    domains
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| s.to_string()).collect()
    }
    #[test]
    fn event_selection_supports_points_only_grants_and_older_hosts_without_widening_scopes() {
        use tiktools_core::collaboration::{Capability, Grant, ResourceKind, StreamDomain};
        let mut grant = Grant {
            capabilities: [Capability::PointsRead, Capability::CommandsRead].into(),
            ..Grant::default()
        };
        let supported = [
            StreamDomain::Workflows,
            StreamDomain::Points,
            StreamDomain::Commands,
        ];
        assert_eq!(
            event_domains(&grant, &supported, |_| true),
            [StreamDomain::Points, StreamDomain::Commands].into()
        );
        assert_eq!(
            event_domains(&grant, &supported, |t| t == "points.changed"),
            [StreamDomain::Points].into()
        );
        assert!(event_domains(
            &grant,
            &[StreamDomain::Workflows, StreamDomain::Commands],
            |t| t == "points.changed"
        )
        .is_empty());
        assert!(event_domains(
            &grant,
            &[StreamDomain::Workflows, StreamDomain::Commands],
            |_| true
        )
        .is_empty());
        grant
            .resources
            .insert(ResourceKind::Live, ["creator".into()].into());
        assert!(event_domains(&grant, &supported, |_| true).is_empty());
        grant.capabilities.insert(Capability::WorkflowsRead);
        assert_eq!(
            event_domains(&grant, &supported, |_| true),
            [StreamDomain::Workflows, StreamDomain::Commands].into()
        );
    }
    #[test]
    fn pairing_parser_accepts_no_secret_argument_or_silent_trust_change() {
        assert!(parse(&args(&[
            "pair",
            "--url",
            "https://host.example",
            "--profile",
            "desk",
            "--secret",
            "private-marker"
        ]))
        .is_err());
        assert!(parse(&args(&["trust", "desk", "--fingerprint", &"a".repeat(64)])).is_err());
        assert!(parse(&args(&[
            "pair",
            "--url",
            "https://host.example",
            "--profile",
            "desk",
            "--profile",
            "other"
        ]))
        .is_err());
        assert!(matches!(
            parse(&args(&[
                "pair",
                "--url",
                "https://host.example",
                "--profile",
                "desk",
                "--invite-stdin"
            ]))
            .unwrap(),
            Plan::Pair { stdin: true, .. }
        ));
    }
    #[test]
    fn invitation_input_is_host_bound_and_errors_do_not_echo_secrets() {
        let id = "00000000-0000-4000-8000-000000000001";
        let secret = "a".repeat(64);
        let input = format!("https://host.example/remote/v1/join#inviteId={id}&secret={secret}");
        assert!(parse_invitation(&input, "https://host.example").is_ok());
        assert!(parse_invitation(
            &json!({"inviteId":id,"secret":secret,"expiresAt":42}).to_string(),
            "https://host.example"
        )
        .is_ok());
        let error = parse_invitation(&input, "https://other.example")
            .err()
            .unwrap();
        assert!(!error.to_string().contains(&secret));
        assert!(
            parse_invitation(&format!("{input}&secret={secret}"), "https://host.example").is_err()
        );
    }
}
