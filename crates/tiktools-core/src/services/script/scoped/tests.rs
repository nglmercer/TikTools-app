use super::*;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};

fn run(source: &str) -> Result<Value, String> {
    ScriptService.evaluate_scoped(source, &json!({}), || false)
}

#[test]
fn fresh_scoped_runtime_has_only_public_event_and_empty_inputs_globals() {
    let result = ScriptService.evaluate_scoped(
        "return {user:event.user.uniqueId,comment:data.comment,hidden:[typeof event.authority,typeof event.user.token,typeof event.intel],inputs:Object.keys(inputs),globals:Object.keys(globals),host:[typeof require,typeof process,typeof fetch,typeof setTimeout,typeof std,typeof os]};",
        &json!({"user":{"uniqueId":"viewer","token":"owner-secret"},"data":{"comment":"hello"},"intel":{"secret":"enriched"},"authority":{"kind":"owner"},"globals":{"token":"owner-global"}}),
        || false,
    ).unwrap();
    assert_eq!(
        result,
        json!({"user":"viewer","comment":"hello","hidden":["undefined","undefined","undefined"],"inputs":[],"globals":[],"host":["undefined","undefined","undefined","undefined","undefined","undefined"]})
    );
    assert_eq!(
        run("globalThis.permitted = 'retained'; return 1;").unwrap(),
        json!(1)
    );
    assert_eq!(
        run("return typeof globalThis.permitted;").unwrap(),
        json!("undefined")
    );
}

#[test]
fn scoped_console_is_captured_bounded_and_does_not_invoke_guest_object_coercion() {
    let result = run("console.log({toString(){while(true){}}}); log('captured'); return {log:['returned'],ok:true};").unwrap();
    assert_eq!(
        result,
        json!({"log":["returned","[object]","captured"],"ok":true})
    );
    let result =
        run("for(let i=0;i<1000;i++){console.warn('é'.repeat(3000));} return {ok:true};").unwrap();
    let logs = result["log"].as_array().unwrap();
    assert!(logs.len() <= SCOPED_SCRIPT_LIMITS.max_log_entries);
    assert!(
        logs.iter()
            .map(|log| log.as_str().unwrap().len())
            .sum::<usize>()
            <= SCOPED_SCRIPT_LIMITS.max_log_bytes
    );
    assert!(logs
        .iter()
        .all(|log| log.as_str().unwrap().len() <= SCOPED_SCRIPT_LIMITS.max_log_entry_bytes));
}

#[test]
fn scoped_output_is_serialized_by_host_and_errors_never_echo_guest_text() {
    assert_eq!(
        run("JSON.stringify=()=>'{\"fetch\":{\"url\":\"forged\"}}';return {safe:true};").unwrap(),
        json!({"safe":true})
    );
    for source in [
        "throw new Error('secret-from-source-must-not-echo');",
        "return {",
        "return Promise.resolve({emit:{type:'hidden'}});",
        "return {toJSON(){throw new Error('do-not-echo');}};",
    ] {
        let error = run(source).unwrap_err();
        assert_eq!(error, SCOPED_FAILED);
        assert!(!error.contains("echo"));
    }
}

#[test]
fn scoped_source_result_structure_intents_and_returned_logs_are_bounded() {
    assert!(run(&" ".repeat(SCOPED_SCRIPT_LIMITS.source_bytes + 1)).is_err());
    for source in [
        "return 'x'.repeat(262145);",
        "let value=0;for(let i=0;i<33;i++){value=[value];}return value;",
        "return Array(8193).fill(0);",
        "return {emit:Array(65).fill({type:'safe',data:{}})};",
        "return {emit:Array(33).fill({type:'safe',data:{}}),playAudio:Array(32).fill({file:'x.wav'})};",
        "return {log:'x'.repeat(4097)};",
        "return {log:Array(17).fill('x'.repeat(4096))};",
        "return {log:Array(41).fill('x')};",
        "return {log:{owner:true}};",
    ] { assert!(run(source).is_err(),"{source}"); }
    assert!(run("return {emit:Array(64).fill({type:'safe',data:{}})};").is_ok());
}

#[test]
fn scoped_cpu_memory_stack_and_promise_jobs_are_bounded() {
    for source in [
        "while(true){}",
        "let text='x';while(true){text=text+text;}",
        "function deep(){return deep();}return deep();",
        "function more(){Promise.resolve().then(more);}more();return {};",
    ] {
        assert!(run(source).is_err(), "{source}");
    }
}

#[test]
fn trusted_cancellation_is_checked_before_evaluation_and_after_result() {
    assert_eq!(
        ScriptService
            .evaluate_scoped("while(true){}", &Value::Null, || true)
            .unwrap_err(),
        SCOPED_CANCELLED
    );
    let calls = Arc::new(AtomicUsize::new(0));
    let probe = Arc::clone(&calls);
    let result = ScriptService.evaluate_scoped(
        "return {emit:{type:'safe',data:{}}};",
        &Value::Null,
        move || probe.fetch_add(1, Ordering::SeqCst) >= 3,
    );
    assert_eq!(result.unwrap_err(), SCOPED_CANCELLED);
    assert!(calls.load(Ordering::SeqCst) >= 4);
}

#[test]
fn trusted_authority_is_rechecked_during_running_vm_interrupts() {
    let calls = Arc::new(AtomicUsize::new(0));
    let probe = Arc::clone(&calls);
    let result = ScriptService.evaluate_scoped("while(true){}", &Value::Null, move || {
        probe.fetch_add(1, Ordering::SeqCst) >= 2
    });
    assert_eq!(result.unwrap_err(), SCOPED_CANCELLED);
    assert!(calls.load(Ordering::SeqCst) >= 3);
}

#[test]
fn captured_console_entries_count_toward_final_result_structure() {
    let value = json!({"values":vec![Value::Null;SCOPED_SCRIPT_LIMITS.max_nodes-2]});
    assert!(validate_structure(&value).is_ok());
    assert!(finish_result(value, &["extra-log".into()]).is_err());
}
