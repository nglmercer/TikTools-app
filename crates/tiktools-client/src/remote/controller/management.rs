//! Closed native management dispatch for profiles, pairing and controller domains.
use super::*;

impl Controller {
    /// Native management RPCs. Their result cannot contain access/refresh tokens.
    pub async fn manage(&self, method: &str, params: Value) -> Result<Value, ClientError> {
        #[cfg(feature = "cloud-peer")]
        if method.starts_with("controller.cloud.") {
            return self.manage_cloud(method, params).await;
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct Pair {
            profile: String,
            origin: String,
            invitation: String,
            label: String,
            fingerprint: Option<String>,
            #[serde(default)]
            confirm_pin: bool,
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Name {
            profile: String,
        }
        match method {
            "controller.drafts.list"=>{
                let p:drafts::Read=parse(params)?;
                if p.id.is_some(){return Err(fail("invalid_params","draft list does not accept an ID"));}
                self.read_drafts(p).await
            },
            "controller.drafts.get"=>{
                let p:drafts::Read=parse(params)?;
                if p.id.is_none()||p.offset!=0{return Err(fail("invalid_params","draft lookup requires one ID"));}
                self.read_drafts(p).await
            },
            "controller.drafts.save"=>self.change_draft(parse(params)?,false).await,
            "controller.drafts.delete"=>self.change_draft(parse(params)?,true).await,
            "controller.points"=>{#[derive(Deserialize)]#[serde(rename_all="camelCase",deny_unknown_fields)]struct P{epoch:String,points_revision:String}let p:P=parse(params)?;self.points(&p.epoch,&p.points_revision)},
            "controller.points.select"=>{#[derive(Deserialize)]#[serde(rename_all="camelCase",deny_unknown_fields)]struct P{epoch:String,unique_id:Option<String>}let p:P=parse(params)?;self.select_points(&p.epoch,p.unique_id)},
            "controller.live.status"=>{#[derive(Deserialize)]#[serde(rename_all="camelCase",deny_unknown_fields)]struct P{epoch:String,connection_id:String}let p:P=parse(params)?;self.live_status(&p.epoch,&p.connection_id).await},
            "controller.live.feed"=>{#[derive(Deserialize)]#[serde(rename_all="camelCase",deny_unknown_fields)]struct P{epoch:String,connection_id:String,after:String,creator_id:Option<String>,stream_id:Option<String>}let p:P=parse(params)?;self.live_feed(&p.epoch,&p.connection_id,&p.after,p.creator_id.as_deref(),p.stream_id.as_deref()).await},
            "controller.competitions"=>{#[derive(Deserialize)]#[serde(rename_all="camelCase",deny_unknown_fields)]struct P{epoch:String,connection_id:String,after:Option<String>}let p:P=parse(params)?;self.competitions(&p.epoch,&p.connection_id,p.after.as_deref()).await},
            "controller.workflow.command"=>self.workflow_command(parse(params)?).await,
            "controller.points.command"=>self.points_command(parse(params)?).await,
            "controller.automation.command"=>self.automation_command(parse(params)?).await,
            "controller.competition.command"=>self.competition_command(parse(params)?).await,
            "controller.proposals.list"=>self.proposal_read(parse(params)?,true).await,
            "controller.proposals.get"=>self.proposal_read(parse(params)?,false).await,
            "controller.proposals.save"=>self.proposal_save(parse(params)?).await,
            "controller.proposals.delete"=>self.proposal_delete(parse(params)?).await,
            "controller.proposals.inspect"=>self.proposal_inspect(parse(params)?).await,
            "controller.proposals.source"=>self.proposal_source(parse(params)?).await,
            "controller.proposals.submit"=>self.proposal_command(parse(params)?).await,
            "controller.automations"=>{#[derive(Deserialize)]#[serde(rename_all="camelCase",deny_unknown_fields)]struct P{epoch:String,automation_revision:String,#[serde(default)]offset:usize}let p:P=parse(params)?;self.automations(&p.epoch,&p.automation_revision,p.offset)},
            "controller.command.resolve"=>{
                #[derive(Deserialize)] #[serde(rename_all="camelCase",deny_unknown_fields)] struct Resolve { epoch:String,command_id:String }
                let p:Resolve=parse(params)?; self.resolve_command(p.epoch,p.command_id).await
            },
            "controller.command.review"=>{
                #[derive(Deserialize)] #[serde(rename_all="camelCase",deny_unknown_fields)] struct Review { epoch:String,command_id:String,acknowledge_unknown:bool }
                let p:Review=parse(params)?;
                if !p.acknowledge_unknown {return Err(fail("confirmation_required","review the unknown outcome explicitly"));}
                self.review_command(p.epoch,p.command_id).await
            },
            "controller.status"=>serde_json::to_value(self.status()).map_err(|_|protocol()),
            "controller.profiles"=>Ok(Value::Array(self.profiles.list().await?.into_iter().map(|p|json!({"name":p.name,"profileId":p.profile_id,"origin":p.origin,"hostId":p.host_id,"trust":p.trust})).collect())),
            "controller.connect"=>{let p:Name=parse(params)?;self.connect(&p.profile).await?;Ok(json!({"ok":true}))},
            "controller.disconnect"=>{self.disconnect().await;Ok(json!({"ok":true}))},
            "controller.profile.forget"=>{
                #[derive(Deserialize)] #[serde(rename_all="camelCase",deny_unknown_fields)]
                struct Forget { profile:String, profile_id:String, origin:String, host_id:String, confirm:bool }
                let p:Forget=parse(params)?;
                if !p.confirm {return Err(fail("confirmation_required","confirm local profile and draft removal"));}
                // Serialize with profile selection, pairing and disconnect.
                let _lifecycle=self.lifecycle.lock().await;
                if self.status().profile.as_deref()==Some(p.profile.as_str()) {
                    return Err(fail("profile_in_use","disconnect this profile before forgetting it"));
                }
                self.profiles.forget_reviewed(&p.profile,&p.profile_id,&p.origin,&p.host_id).await?;
                Ok(json!({"ok":true}))
            },
            "controller.probe"=>{#[derive(Deserialize)] #[serde(deny_unknown_fields)] struct Probe{origin:String} let p:Probe=parse(params)?;Ok(json!({"fingerprint":probe_fingerprint(&p.origin).await?,"verified":false}))},
            "controller.pair"=>{
                let mut p:Pair=parse(params)?;
                let private=zeroize::Zeroizing::new(std::mem::take(&mut p.invitation));
                let trust=match p.fingerprint {Some(pin) if p.confirm_pin=>TlsTrust::CertificateSha256{sha256:normalize_fingerprint(&pin)?},Some(_)=>return Err(fail("confirmation_required","verify the host fingerprint before approving it")),None=>TlsTrust::System};
                let mut l=self.lifecycle.lock().await;
                if self.status().phase!="idle" && self.status().phase!="failed" {return Err(fail("invalid_state","disconnect before pairing another host"));}
                self.profiles.ensure_new(&p.profile).await?;self.profiles.check_store().await?;
                let endpoint=RemoteEndpoint::new(&p.origin,trust).await?;
                let (id,secret)=parse_invitation(&private,endpoint.origin())?;
                let pending=endpoint.begin_pairing(&id,&secret,&p.label,ClientKind::Desktop).await?;
                let result=json!({"requestId":pending.request_id,"expiresAt":pending.expires_at});
                let epoch=self.reset(&mut l);
                self.shared.change(&epoch,|v|{v.status.phase="pairing".into();v.status.pairing_request_id=Some(pending.request_id.clone());v.status.profile=Some(p.profile.clone());v.status.origin=Some(endpoint.origin().into());});
                l.pairing=Some(Pairing{profile:p.profile,endpoint,pending});Ok(result)
            },
            "controller.pair.poll"=>{
                let mut l=self.lifecycle.lock().await;
                let mut pairing=l.pairing.take().ok_or_else(||fail("invalid_state","no pairing request is pending"))?;
                let epoch=self.status().epoch;
                match pairing.pending.poll().await {
                    Ok(PairingPoll::Pending)=>{l.pairing=Some(pairing);Ok(json!({"approved":false}))},
                    Ok(PairingPoll::Approved(credentials))=>{
                        let result=self.profiles.install(&pairing.profile,&pairing.endpoint,credentials,ClientKind::Desktop).await;
                        match result {Ok(_)=>{self.shared.phase(&epoch,"idle",None);Ok(json!({"approved":true,"profile":pairing.profile}))},Err(e)=>{self.shared.phase(&epoch,"failed",Some(&e));Err(e)}}
                    },
                    Err(e)=>{self.shared.phase(&epoch,"failed",Some(&e));Err(e)}
                }
            },
            _=>Err(fail("method_not_found","unknown native controller operation"))
        }
    }
}

fn parse<T: serde::de::DeserializeOwned>(p: Value) -> Result<T, ClientError> {
    serde_json::from_value(p).map_err(|_| fail("invalid_params", "invalid controller parameters"))
}
