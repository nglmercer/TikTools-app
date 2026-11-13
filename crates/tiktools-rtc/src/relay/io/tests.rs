use super::*;

fn setup() -> (Stream, Records, mpsc::Receiver<Value>) {
    let local = Context {
        host_id: Uuid::new_v4(),
        session_id: Uuid::new_v4(),
        host_device_id: Uuid::new_v4(),
        from_device_id: Uuid::new_v4(),
        negotiation_id: Uuid::new_v4(),
        grant_revision: 1,
    };
    let mut remote = local.clone();
    remote.from_device_id = local.host_device_id;
    let (outgoing, rx) = mpsc::channel(64);
    let (stream, records) = stream(local, remote, outgoing).unwrap();
    (stream, records, rx)
}
fn data(records: &Records, sequence: i64, bytes: &[u8]) -> Record {
    Record::Data {
        tunnel_id: records.remote.negotiation_id,
        session_id: records.remote.session_id,
        from_device_id: records.remote.from_device_id,
        sequence,
        data: URL_SAFE_NO_PAD.encode(bytes),
    }
}

#[tokio::test]
async fn an_ordered_close_drains_prior_records_before_reaching_eof() {
    let (mut stream, mut records, _rx) = setup();
    records.apply(data(&records, 0, b"last record")).unwrap();
    records
        .apply(Record::Close {
            tunnel_id: records.remote.negotiation_id,
            session_id: records.remote.session_id,
            from_device_id: records.remote.from_device_id,
            sequence: 1,
        })
        .unwrap();
    drop(records);
    let mut bytes = Vec::new();
    tokio::time::timeout(
        std::time::Duration::from_secs(1),
        stream.read_to_end(&mut bytes),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(bytes, b"last record");
}

#[tokio::test]
async fn reordered_or_foreign_records_close_the_stream() {
    for field in ["tunnelId", "sessionId", "fromDeviceId", "sequence"] {
        let (mut stream, mut records, _rx) = setup();
        let mut value = json!({"type":"relay.data","tunnelId":records.remote.negotiation_id,
            "sessionId":records.remote.session_id,"fromDeviceId":records.remote.from_device_id,
            "sequence":0,"data":URL_SAFE_NO_PAD.encode(b"record")});
        value[field] = if field == "sequence" {
            json!(1)
        } else {
            json!(Uuid::new_v4())
        };
        assert!(records
            .apply(serde_json::from_value(value).unwrap())
            .is_err());
        assert!(records.is_closed());
        let mut bytes = [0; 1];
        assert_eq!(stream.read(&mut bytes).await.unwrap(), 0);
    }
}

#[tokio::test]
async fn malformed_or_oversized_record_payloads_are_fatal() {
    for payload in [
        String::new(),
        "not!base64".into(),
        URL_SAFE_NO_PAD.encode(vec![0; RECORD_BYTES + 1]),
    ] {
        let (mut stream, mut records, _rx) = setup();
        let Record::Data {
            tunnel_id,
            session_id,
            from_device_id,
            sequence,
            ..
        } = data(&records, 0, b"record")
        else {
            unreachable!()
        };
        assert!(records
            .apply(Record::Data {
                tunnel_id,
                session_id,
                from_device_id,
                sequence,
                data: payload
            })
            .is_err());
        let mut bytes = [0; 1];
        assert_eq!(stream.read(&mut bytes).await.unwrap(), 0);
    }
}

#[tokio::test]
async fn overflow_or_owner_drop_cancels_the_bounded_bridge() {
    let (stream, mut records, mut rx) = setup();
    // No await: the pump has not yet drained this bounded queue.
    for sequence in 0..RECORD_QUEUE {
        records
            .apply(data(&records, sequence as i64, b"record"))
            .unwrap();
    }
    assert!(records
        .apply(data(&records, RECORD_QUEUE as i64, b"overflow"))
        .is_err());
    assert!(records.is_closed());
    assert!(rx.recv().await.is_none());
    drop(stream);
    let (stream, records, mut rx) = setup();
    drop(stream);
    assert!(records.is_closed());
    assert!(rx.recv().await.is_none());
}
