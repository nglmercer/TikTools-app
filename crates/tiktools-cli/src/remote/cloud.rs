//! Cloud P2P onboarding in one native process, including memory-only login.
use super::*;
use tiktools_client::remote::cloud_peer::CloudPeerController;

#[derive(Debug, PartialEq)]
enum Command {
    Status,
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
    },
    Pair {
        name: String,
        label: String,
        kind: ClientKind,
        stdin: bool,
    },
}
fn parse(args: &[String]) -> Result<Command, String> {
    let verb = args
        .first()
        .map(String::as_str)
        .ok_or("use remote cloud status|login|logout|enroll|join|connect|pair")?;
    match verb {
        "status" | "login" | "logout" if args.len() == 1 => Ok(match verb {
            "status" => Command::Status,
            "login" => Command::Login,
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
                    &["--profile", "--kind", "--label"]
                } else {
                    &["--profile", "--kind", "--host"]
                },
                if verb == "pair" {
                    &["--invite-stdin"]
                } else {
                    &[]
                },
            )?;
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
                })
            } else {
                let host = opts.remove("--host").ok_or("--host is required")?;
                if uuid_check(&host) {
                    Ok(Command::Connect { name, host, kind })
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

pub(super) async fn run(args: &[String], json_mode: bool) -> i32 {
    let command = match parse(args) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("tiktools: {error}");
            return 2;
        }
    };
    match execute(command).await {
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
fn native_auth(failure: impl std::fmt::Display) -> ClientError {
    error("cloud_auth", &failure.to_string())
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
            return serde_json::to_value(controller.account().status().await)
                .map_err(|_| error("protocol", "invalid account status"))
        }
        Command::Logout => {
            return serde_json::to_value(controller.account().logout().await.map_err(native_auth)?)
                .map_err(|_| error("protocol", "invalid account status"))
        }
        Command::Login => {
            login(&controller).await?;
            return serde_json::to_value(controller.account().status().await)
                .map_err(|_| error("protocol", "invalid account status"));
        }
        _ => {}
    }
    if let Err(restore) = controller.account().restore().await {
        // Pair keeps login, enrollment and admission in one process, so it
        // also works when the OS can provide only memory-only persistence.
        if matches!(command, Command::Pair { .. })
            && matches!(
                restore.code(),
                "no-saved-session" | "expired" | "storage-unavailable"
            )
        {
            login(&controller).await?;
        } else {
            return Err(native_auth(restore));
        }
    }
    match command {
        Command::Enroll { label } => serde_json::to_value(controller.enroll(label).await?)
            .map_err(|_| error("protocol", "invalid enrolled device")),
        Command::Join { stdin } => {
            let token = tokio::task::spawn_blocking(move || private_invitation(stdin))
                .await
                .map_err(|_| error("input", "private invitation input failed"))??;
            serde_json::to_value(controller.redeem(token.to_string()).await?)
                .map_err(|_| error("protocol", "invalid join request"))
        }
        Command::Connect { name, host, .. } => {
            serde_json::to_value(controller.connect(&name, &host).await?)
                .map_err(|_| error("protocol", "invalid peer profile"))
        }
        Command::Pair {
            name, label, stdin, ..
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
            eprintln!(
                "Waiting for owner approval of request {} (host {})",
                request.request_id, request.host_id
            );
            tokio::time::timeout(Duration::from_secs(600), async {
                loop {
                    match controller.connect(&name, &request.host_id).await {
                        Ok(profile) => {
                            return serde_json::to_value(profile)
                                .map_err(|_| error("protocol", "invalid peer profile"))
                        }
                        Err(error)
                            if matches!(
                                error.code.as_str(),
                                "approval_pending"
                                    | "transport"
                                    | "unauthorized"
                                    | "unavailable"
                                    | "rate_limited"
                            ) =>
                        {
                            tokio::time::sleep(Duration::from_secs(10)).await
                        }
                        Err(error) => return Err(error),
                    }
                }
            })
            .await
            .map_err(|_| {
                error(
                    "approval_pending",
                    "approval timed out; use remote cloud connect with the host ID shown above",
                )
            })?
        }
        _ => unreachable!(),
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
