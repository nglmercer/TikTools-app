---
title: "Plugin packaging, signing and registries"
docId: "developer.plugins.packaging-registry"
translationKey: "developer.plugins.packaging-registry"
locale: "en"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "951946c8afe780a0fa42a1ee78e8962cdda20b38ebd69332cbdb9e77d9c34acf"
translationRevision: "951946c8afe780a0fa42a1ee78e8962cdda20b38ebd69332cbdb9e77d9c34acf"
order: 280
---

# Plugin packaging, signing and registries

## Goal and prerequisites

Distribute a package whose bytes, publisher authority and host compatibility can be checked independently. Start with a validated plugin and a supported target. Signing needs private author keys kept outside the repository; registry root delegation is a separate operator responsibility.

## Prepare a reviewable archive

```sh
tiktools-plugin validate --publish
tiktools-plugin build --release
tiktools-plugin keygen --id acme-1 --publisher acme --out /private/path/keys
```

`build` compiles the package and creates `.plugin` archives. Manifest entry, assets, locales, UI output and relevant runtime dependencies are included with `checksums.json`. Native/process builds need a compatible target artifact; JavaScript-only packages remain portable until native addons or platform facilities constrain them.

For an existing built archive, use exact file paths rather than an unreviewed wildcard:

```sh
tiktools-plugin sign dist-plugin/acme.hello-0.1.0.plugin --key /private/path/keys/acme-1.key.json
tiktools-plugin verify dist-plugin/acme.hello-0.1.0.plugin --trust /private/path/keys/acme-1.pub.json
```

Replace the archive name with the build's actual target-specific output. The signature covers checksums, which cover package files. Public keys can be shared; private key files must not enter source control or logs.

## Trust and update policy

The static registry protocol has root-signed authority and delegated index/publisher keys. Publisher namespace approval is not the same as package integrity, and a Cloud catalog digest cannot replace either signature chain. The resolver checks channel, engine, schema/protocol/ABI, runtime and target. Yanked, revoked or incompatible releases remain ineligible even when their file hash matches.

Host installation first presents a review. Automatic updates require permitted policy, an unpinned package, compatible signed publisher and no new authority; expanded permissions require review. Product Cloud catalog installation is distinct from low-level static registry authoring, and ordinary product flows reject arbitrary remote package URLs.

## Expected result, recovery and verification

The installer can verify bytes before applying a reviewed package and retain rollback/recovery state. Development staging is outside installer management. On a trust failure, repair delegation or publisher metadata rather than use unsigned override flags as normal distribution policy. On uncertain runtime startup, stop replacement and reconcile its retained outcome.

Use `bun scripts/task.ts rust test -p tiktools-plugin-package --locked` and the registry tests for offline fixtures. The [package verifier](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-plugin-package/src) and [registry model](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-plugin-registry/src/model.rs) own the technical formats; publishing infrastructure needs separate operator acceptance.
