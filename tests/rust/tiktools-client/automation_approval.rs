use serde_json::json;

use tiktools_client::automation_approval::ApprovalActivateReceipt;
use tiktools_client::CallResult;

fn activation() -> CallResult<ApprovalActivateReceipt> {
    CallResult {
        result: serde_json::from_value(json!({
            "kind":"event","id":"draft","enabled":true,"approved":true,
            "policyVersion":1,"manifestDigest":"b".repeat(64),"approvalGeneration":"7"
        }))
        .unwrap(),
        meta: Some(
            serde_json::from_value(json!({
                "hostInstanceId":"instance-original",
                "command":{"commandId":"approval-original","state":"succeeded",
                    "committedAt":42,"resource":{"kind":"automation","scope":"event","id":"draft"}},
                "resourceVersions":[
                    {"kind":"automation","scope":"event","id":"draft","revision":"5"},
                    {"kind":"automation","scope":"action","id":"draft","revision":"3"},
                    {"kind":"automation","scope":"action","id":"inactive","revision":"3"}
                ]
            }))
            .unwrap(),
        ),
    }
}

#[test]
fn receipt_shapes_reject_additional_authority_or_source_data() {
    let original = serde_json::to_value(activation().result).unwrap();
    for field in [
        "record",
        "source",
        "authority",
        "provenance",
        "grant",
        "permit",
    ] {
        let mut extra = original.clone();
        extra[field] = json!({});
        assert!(serde_json::from_value::<ApprovalActivateReceipt>(extra).is_err());
    }
}
