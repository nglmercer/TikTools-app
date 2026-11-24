use semver::Version;
use tiktools_app_update::Channel;

#[test]
fn channels_reject_downgrades_and_stable_rejects_prereleases() {
    let v = |s| Version::parse(s).unwrap();
    assert!(!Channel::Stable.permits(&v("4.0.0"), &v("4.0.0")));
    assert!(!Channel::Beta.permits(&v("4.0.0"), &v("3.9.9")));
    assert!(!Channel::Stable.permits(&v("4.0.0"), &v("4.1.0-beta.1")));
    assert!(Channel::Beta.permits(&v("4.0.0"), &v("4.1.0-beta.1")));
    assert!(Channel::Beta.permits(&v("4.1.0-beta.1"), &v("4.1.0")));
}
