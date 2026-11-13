---
title: "ルールテンプレート・プロファイル・所有者確認"
docId: "developer.templates"
translationKey: "developer.templates"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "51b5b2821344dd1795acb1008496542c90800461cc5fcfff5f2ed41aef6ec7b1"
translationRevision: "51b5b2821344dd1795acb1008496542c90800461cc5fcfff5f2ed41aef6ec7b1"
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

確認済み取り込みは整合した関連レコードを作り、所有者ジャーナルを保持します。Cloud カタログの取り込みは所有者が意図して有効にするまで無効です。提供元が不足・無効なら別アクションへ黙って置換せず要件エラーになります。選択した1つのプロファイルパックが表示対象を制御し、プロファイル選択でリモート権限は得られません。

## 復旧と検証 {#recover-and-verify}

競合では候補を保持し、ライブラリのスナップショットを再読込して実差分を確認します。別の取り込み意図を作る前に保留ジャーナルを解決します。更新失敗を暗黙の新規作成に変えません。対象検証には `bun scripts/task.ts test web rule-templates` と `bun scripts/task.ts test web library-journal` を使います。[ネイティブサービス](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/services/templates.rs) と生成 RPC スキーマが、旧 `src/web/views/behavior` 参照に優先します。
