---
title: "CLI コマンドリファレンス"
docId: "developer.cli"
translationKey: "developer.cli"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6bb485d6e8067cb1639fb20821c45b79941245692f0b6c7525f2b3bee930048b"
translationRevision: "99242db6ea6c989f5eb459b10fb84a98a8bb5499e2e2d6bd73d7b00d32be8b4d"
order: 75
---

# CLI コマンドリファレンス {#cli-command-reference}

## 目的と前提条件 {#goal-and-prerequisites}

Rust のソースを読んだり人間向けのヘルプ出力を解析したりせずに、すべての `tiktools` コマンド形式を発見します。一致する `tiktools` バイナリがビルドされている必要があります。このリファレンスはオフラインで、実行中の host を必要としません。正規のソースは `crates/tiktools-cli/src/commands/spec.rs` のコマンド仕様レジストリです。各ドメインモジュールはパーサーがエラーで報告する使用法文字列を所有しているため、このリファレンスはパーサーが受け入れるものから逸脱しません。

## オフラインでヘルプを検査 {#inspect-help-offline}

人間向けヘルプはドメインごとに動詞をグループ化します。フィルターを指定すると1つのドメインセクションが表示されます。`--json` 形式は、すべてのエージェントが優先すべき機械可読レジストリを出力します。動詞ごとの使用法、要約、破壊的マーカーに加え、グローバルな CLI 規約が含まれ、トップレベルの `schema` フィールドでバージョン管理されています。

```sh
tiktools help
tiktools help plugin
tiktools help --json
```

使用法エラーはページ全体ではなく、関連する形式とヒントのみを出力するため、タイプミスでターミナルがスクロールすることはありません。終了コード 2 は常に使用法エラーを意味します。host 解決、出力、シークレット秘匿のルールについては、規約のフッターを参照してください。

## 生成されたコマンドリファレンス {#generated-command-reference}

`docs/contracts/cli-help.json` にチェックインされた CLI ヘルプスナップショットから生成されます。バイナリが権威であり続けます。使用法行はパーサーの構文であり、すべてのロケールで英語のままです。要約と規約は CLI 自身の翻訳カタログから来るため、このリファレンスは `tiktools help` が同じ言語で出力する内容から逸脱しません。破壊的とマークされた動詞は状態を変更または破棄します。スクリプトは実行前に確認すべきです。

<!--@include: ../../../.generated/cli.ja.md-->

## 運用上の詳細 {#operational-details}

スナップショットは `--features local-web` でビルドされたバイナリから再生成され、フィーチャーゲートされたドメイン（cloud、publisher、web）が完全であることを保証します。`docs check` のプロベナンスチェックはすべての Rust ソースのハッシュを比較するため、コマンド仕様の変更はコントラクトが再生成されるまでビルドを失敗させます。通常のドキュメントビルドで Cargo が実行されることはありません。

```sh
bun docs/scripts/regenerate-control.ts
bun docs/scripts/generate-references.ts --check
bun docs/scripts/cli-coverage.ts
```

コマンドパーサーまたは仕様を変更した後に再生成を実行し、変更の一部としてフラグメントの diff を確認してください。`help --json` を消費するエージェントは、スキーマバージョン 1 を加法的に安定しているものとして扱うべきです。新しい動詞、ドメイン、フィールドが予告なく現れることがありますが、既存のキーはその意味を保持します。
