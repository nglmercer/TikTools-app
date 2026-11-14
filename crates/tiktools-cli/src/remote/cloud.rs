//! Cloud P2P onboarding in one native process, including memory-only login.
use super::*;
use tiktools_client::remote::cloud_peer::CloudPeerController;

#[derive(Debug, PartialEq)]
enum Command {
    Status,
    Teams,
    Doctor,
    Login,
    Logout,
    Enroll {
        label: String,
    },
    Join {
        stdin: bool,
    },
    Connect {
        name: String,
        host: String,
        kind: ClientKind,
        budget: Duration,
        wait: bool,
    },
    Pair {
        name: String,
        label: String,
        kind: ClientKind,
        stdin: bool,
        budget: Duration,
        wait: bool,
        non_interactive: bool,
    },
}
fn parse(args: &[String]) -> Result<Command, String> {
    let verb = args
        .first()
        .map(String::as_str)
        .ok_or("use remote cloud status|login|logout|enroll|join|connect|pair")?;
    match verb {
        "status" | "login" | "logout" | "teams" | "doctor" if args.len() == 1 => Ok(match verb {
            "status" => Command::Status,
            "login" => Command::Login,
            "teams" => Command::Teams,
            "doctor" => Command::Doctor,
            _ => Command::Logout,
        }),
        "enroll" => {
            let mut opts = options(&args[1..], &["--label"], &[])?;
            Ok(Command::Enroll {
                label: opts
                    .remove("--label")
                    .unwrap_or_else(|| "TikTools controller".into()),
            })
        }
        "join" => {
            let opts = options(&args[1..], &[], &["--invite-stdin"])?;
            Ok(Command::Join {
                stdin: opts.contains_key("--invite-stdin"),
            })
        }
        "connect" | "pair" => {
            let mut opts = options(
                &args[1..],
                if verb == "pair" {
                    &["--profile", "--kind", "--label", "--timeout"]
                } else {
                    &["--profile", "--kind", "--host", "--timeout"]
                },
                if verb == "pair" {
                    &["--invite-stdin", "--wait", "--no-wait", "--non-interactive"]
                } else {
                    &["--wait", "--non-interactive"]
                },
            )?;
            let budget =
                crate::commands::teams::timeout(&opts, if verb == "pair" { 600 } else { 90 })?;
            if opts.contains_key("--wait") && opts.contains_key("--no-wait") {
                return Err("--wait and --no-wait are mutually exclusive".into());
            }
            let non_interactive = opts.contains_key("--non-interactive");
            if verb == "pair" && non_interactive && !opts.contains_key("--invite-stdin") {
                return Err("--non-interactive requires --invite-stdin".into());
            }
            let wait = opts.contains_key("--wait")
                || verb == "pair" && !non_interactive && !opts.contains_key("--no-wait");
            let name = opts.remove("--profile").ok_or("--profile is required")?;
            if !tiktools_client::remote::profiles::valid_name(&name) {
                return Err("invalid remote profile name".into());
            }
            let kind = match opts.remove("--kind").as_deref().unwrap_or("desktop") {
                "desktop" => ClientKind::Desktop,
                "agent" => ClientKind::Agent,
                _ => return Err("--kind must be desktop or agent".into()),
            };
            if verb == "pair" {
                Ok(Command::Pair {
                    name,
                    label: opts
                        .remove("--label")
                        .unwrap_or_else(|| "TikTools controller".into()),
                    kind,
                    stdin: opts.contains_key("--invite-stdin"),
                    budget,
                    wait,
                    non_interactive,
                })
            } else {
                let host = opts.remove("--host").ok_or("--host is required")?;
                if uuid_check(&host) {
                    Ok(Command::Connect {
                        name,
                        host,
                        kind,
                        budget,
                        wait,
                    })
                } else {
                    Err("invalid Cloud host ID".into())
                }
            }
        }
        _ => Err("use remote cloud status|login|logout|enroll|join|connect|pair".into()),
    }
}
fn uuid_check(raw: &str) -> bool {
    raw.len() == 36
        && raw.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        })
}

pub(crate) async fn run(args: &[String], json_mode: bool) -> i32 {
    let command = match parse(args) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("tiktools: {error}");
            return 2;
        }
    };
    let budget = match &command {
        Command::Pair { budget, .. } | Command::Connect { budget, .. } => *budget,
        _ => Duration::from_secs(120),
    };
    let execution = tokio::time::timeout(budget, execute(command))
        .await
        .unwrap_or_else(|_| {
            Err(error(
                "timeout",
                "Cloud onboarding deadline exceeded; an accepted join request may still exist",
            ))
        });
    match execution {
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
fn native_auth(failure: tiktools_core::cloud_account::AuthError) -> ClientError {
    error(failure.code(), &failure.to_string())
}
async fn login(controller: &CloudPeerController) -> Result<(), ClientError> {
    let begin = controller
        .account()
        .begin_login()
        .await
        .map_err(native_auth)?;
    eprintln!(
        "Open this URL in your browser to sign in:\n{}",
        begin.browser_url
    );
    controller
        .account()
        .complete_login()
        .await
        .map_err(native_auth)?;
    Ok(())
}
async fn execute(command: Command) -> Result<Value, ClientError> {
    let profiles = ProfileManager::system()?;
    let kind = match &command {
        Command::Connect { kind, .. } | Command::Pair { kind, .. } => *kind,
        _ => ClientKind::Desktop,
    };
    let controller = CloudPeerController::from_environment(
        profiles.directory().join("cloud-account"),
        profiles.clone(),
        kind,
    )?;
    match command {
        Command::Status => {
            let restored = controller.account().restore().await;
            let mut status = serde_json::to_value(controller.account().status().await)
                .map_err(|_| error("protocol", "invalid account status"))?;
            status["context"] = json!("controller");
            status["verified"] = json!(restored.is_ok());
            status["credentialState"] =
                json!(
                    match restored.as_ref().err().map(|failure| failure.code()) {
                        None => "verified",
                        Some("no-saved-session" | "unconfigured") => "missing",
                        Some("storage-unavailable") => "unavailable",
                        Some("expired") => "expired",
                        Some(_) => "unverified",
                    }
                );
            if let Err(failure) = restored {
                status["restoreError"] = json!({"code":failure.code()});
            }
            return Ok(status);
        }
        Command::Doctor => {
            let restored = controller.account().restore().await;
            let status = controller.account().status().await;
            let teams = if restored.is_ok() {
                Some(controller.memberships().await?)
            } else {
                None
            };
            return Ok(
                json!({"context":"controller","verified":restored.is_ok(),"account":status,"teams":teams,"restoreError":restored.err().map(|e| e.code().to_owned())}),
            );
        }
        Command::Login => {
            login(&controller).await?;
            return serde_json::to_value(controller.account().status().await)
                .map_err(|_| error("protocol", "invalid account status"));
        }
        Command::Logout => {
            return serde_json::to_value(controller.account().logout().await.map_err(native_auth)?)
                .map_err(|_| error("protocol", "invalid account status"))
        }
        _ => {}
    }
    if let Err(restore) = controller.account().restore().await {
        // Pair keeps login, enrollment and admission in one process, so it
        // also works when the OS can provide only memory-only persistence.
        if matches!(
            command,
            Command::Pair {
                non_interactive: false,
                ..
            }
        ) && matches!(
            restore.code(),
            "no-saved-session" | "expired" | "storage-unavailable"
        ) {
            login(&controller).await?;
        } else {
            let mut failure = error(
                "authentication_required",
                "Cloud authentication is required for the controller context",
            );
            failure.data = Some(
                json!({"context":"controller","cause":restore.code(),"nextCommand":"tiktools remote cloud login"}),
            );
            return Err(failure);
        }
    }
    match command {
        Command::Teams => {
            Ok(json!({"context":"controller","items":controller.memberships().await?}))
        }
        Command::Enroll { label } => serde_json::to_value(controller.enroll(label).await?)
            .map_err(|_| error("protocol", "invalid enrolled device")),
        Command::Join { stdin } => {
            let token = tokio::task::spawn_blocking(move || private_invitation(stdin))
                .await
                .map_err(|_| error("input", "private invitation input failed"))??;
            serde_json::to_value(controller.redeem(token.to_string()).await?)
                .map_err(|_| error("protocol", "invalid join request"))
        }
        Command::Connect {
            name, host, wait, ..
        } => connect(&controller, &name, &host, wait).await,
        Command::Pair {
            name,
            label,
            stdin,
            wait,
            ..
        } => {
            profiles.ensure_new(&name).await?;
            match controller.controller_device_id().await {
                Ok(_) => {}
                Err(missing) if missing.code == "invalid_params" => {
                    controller.enroll(label).await?;
                }
                Err(error) => return Err(error),
            }
            let token = tokio::task::spawn_blocking(move || private_invitation(stdin))
                .await
                .map_err(|_| error("input", "private invitation input failed"))??;
            let request = controller.redeem(token.to_string()).await?;
            if !wait {
                return Ok(
                    json!({"state":"approval_pending","requestId":request.request_id,"hostId":request.host_id,"nextCommand":format!("tiktools remote cloud connect --host {} --profile {}",request.host_id,name)}),
                );
            }
            connect(&controller, &name, &request.host_id, true).await
        }
        _ => unreachable!(),
    }
}

async fn connect(
    controller: &CloudPeerController,
    name: &str,
    host: &str,
    wait: bool,
) -> Result<Value, ClientError> {
    loop {
        match controller.connect(name, host).await {
            Ok(profile) => {
                return serde_json::to_value(profile)
                    .map_err(|_| error("protocol", "invalid peer profile"))
            }
            Err(failure)
                if wait
                    && matches!(
                        failure.code.as_str(),
                        "approval_pending" | "transport" | "unavailable" | "rate_limited"
                    ) =>
            {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            Err(failure) => return Err(failure),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| (*v).into()).collect()
    }
    #[test]
    fn cloud_onboarding_rejects_secret_arguments_and_invalid_native_targets() {
        assert!(parse(&args(&["pair", "--profile", "agent", "--invite", "secret"])).is_err());
        assert!(parse(&args(&["connect", "--profile", "agent", "--host", "wrong"])).is_err());
        assert!(parse(&args(&["pair", "--profile", "agent", "--kind", "browser"])).is_err());
        assert!(matches!(
            parse(&args(&[
                "pair",
                "--profile",
                "agent",
                "--kind",
                "agent",
                "--invite-stdin"
            ])),
            Ok(Command::Pair {
                kind: ClientKind::Agent,
                stdin: true,
                ..
            })
        ));
    }
}
