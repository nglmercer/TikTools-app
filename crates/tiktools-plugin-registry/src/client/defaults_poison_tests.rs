use super::*;

#[test]
fn poisoned_configuration_never_yields_permissive_security_defaults() {
    let root =
        std::env::temp_dir().join(format!("registry-poison-{}-{}", std::process::id(), now()));
    let client = RegistryClient::new_fixed(
        root.join("cache"),
        root.join("registries.json"),
        Vec::new(),
        Arc::new(RwLock::new(TrustStore::new())),
    )
    .unwrap();
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut config = client.config.write().unwrap();
        config.defaults.allow_sideload = false;
        panic!("simulate interrupted settings mutation");
    }))
    .is_err());
    assert!(client.try_defaults().is_err());
}
