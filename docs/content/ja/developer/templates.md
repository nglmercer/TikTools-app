---
title: "ルールテンプレート・プロファイル・所有者確認"
docId: "developer.templates"
translationKey: "developer.templates"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "34c35d378f38a78866661b97c19113f83b5b5f2846cefa2b5076eb20fe9bdc21"
translationRevision: "6f74dbb9d694c38c0d96ab8399d2de653bb464ddaf849930cea9d7d975b98c35"
order: 160
---

# ルールテンプレート・プロファイル・所有者確認 {#rule-templates-profiles-and-owner-review}

## 目的と前提条件 {#goal-and-prerequisites}

ローカル所有権と復旧性を維持して再利用ルールを取り込みます。非公開ホスト、必要なインストール済みアクション提供元、有効な `.tiktemplate.json` または `.tikprofile.json` を用意します。有効化前に[自動化](automations.md) を読みます。ライブラリ取り込みは設定作業であり、外部連携への到達確認ではありません。

## 文書を準備する {#prepare-a-document}

v1 テンプレートには `templateVersion`、可搬 ID、パラメータースキーマ、アクション、関連イベント1つがあります。プロファイルは `profileVersion`、共有パラメーター、埋め込みテンプレートを持ちます。正規パーサーは `crates/tiktools-core/src/services/templates/` と `src/web/entities/rule-template/lib/`、両側の整合性は `tests/fixtures/templates/` で維持します。

パラメーターはスキーマ既定、プロファイル、項目、明示的なダイアログ・CLI 値の順に統合します。値全体の <code v-pre>{{ params.* }}</code> は JSON 型を保持し、文章内の参照は文字列になります。イベント置換は実行時に残します。実行時グローバルは後から解決するため、アカウントの秘密を共有文書へ焼き込みません。

## プレビューとインポート {#preview-and-import}

```sh
tiktools template import rules.tiktemplate.json --dry-run --param giftName=Rose
tiktools template import rules.tiktemplate.json --disabled --param giftName=Rose
tiktools template list
tiktools template profile-export --out backup.tikprofile.json
```

例ではファイルを準備済みで対応ホストが稼働していることを前提にします。UI のルールテンプレートとプロファイル取り込みは候補を準備して確認します。ネイティブの `automation.library.*` コーディネーターが原子的なライブラリ遷移と元の結果の復旧を所有します。現行スキーマは [Control API](control-api.md) で検出できます。

## 期待される結果と提供状況 {#expected-result-and-status}

確認済み取り込みは整合した関連レコードを作り、所有者ジャーナルを保持します。Cloud カタログは文書を保存し、プロファイルの取り込み・後の適用では無効と仮定せず enabled 値を確認します。提供元が不足・無効なら別アクションへ黙って置換せず要件エラーになります。選択した1つのプロファイルパックが表示対象を制御し、プロファイル選択でリモート権限は得られません。

## 復旧と検証 {#recover-and-verify}

競合では候補を保持し、ライブラリのスナップショットを再読込して実差分を確認します。別の取り込み意図を作る前に保留ジャーナルを解決します。更新失敗を暗黙の新規作成に変えません。対象検証には `bun scripts/task.ts test web rule-templates` と `bun scripts/task.ts test web library-journal` を使います。[ネイティブサービス](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/services/templates.rs) と生成 RPC スキーマが、旧 `src/web/views/behavior` 参照に優先します。

## 運用の詳細 {#operational-details}

最初の JSON を `docs-log.tiktemplate.json` として保存します。2 番目は 1 項目の完全な `docs-pack.tikprofile.json` です。両方アクション・イベントを明示的に無効化します。優先順は schema 既定 → プロファイル → 項目 → CLI・ダイアログ明示値です。全体置換は型を保ち、埋め込みは文字列になります。enabled 省略は true なので `--disabled` と UI 最終比較を使います。

現在はテンプレートあたり 8 アクション・12 フィルター、プロファイルあたり 32 テンプレート。UI 原文は 64 KiB/テンプレート・256 KiB/プロファイル、展開は深さ 64・65,536 ノード・256 KiB/レコード・総計 8 MiB です。超過は内容を減らして解決し検証を回避しません。ライブラリ・原文・展開の予算は別です。結果不明時は次の import 前に[ジャーナル回復](automations.md#operational-details)を行います。

```json
{"templateVersion":1,"id":"docs-log","title":"Docs log","params":{"type":"object","properties":{"message":{"type":"string","default":"hello"}}},"actions":[{"name":"Log","typeId":"core.log","enabled":false,"config":{"message":"{{ params.message }}"}}],"event":{"name":"Chat","trigger":"tiktok.chat","enabled":false,"filters":[],"cooldownMs":1000,"cooldownScope":"global","runMode":"all"}}
```

```json
{"profileVersion":1,"id":"docs-pack","name":"Docs pack","description":"","params":{"message":"profile"},"templates":[{"params":{"message":"entry"},"template":{"templateVersion":1,"id":"docs-log","title":"Docs log","params":{"type":"object","properties":{"message":{"type":"string","default":"hello"}}},"actions":[{"name":"Log","typeId":"core.log","enabled":false,"config":{"message":"{{ params.message }}"}}],"event":{"name":"Chat","trigger":"tiktok.chat","enabled":false}}}]}
```

```sh
tiktools template import docs-log.tiktemplate.json --dry-run
tiktools template import docs-log.tiktemplate.json --disabled
tiktools template profile-import docs-pack.tikprofile.json --disabled
```
