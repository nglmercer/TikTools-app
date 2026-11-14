---
title: "自動化の実行とスクリプトの契約"
docId: "developer.automations"
translationKey: "developer.automations"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "57cc664e4ef2ff26fafb31df6458da439f075c673fbb3e06e635fa03c1032f42"
translationRevision: "42835015c312a2f0ab5603c7406b60b50850ad8b577c1ee531b27468dcfeb386"
order: 150
---

# 自動化の実行とスクリプトの契約 {#automation-execution-and-script-contracts}

## 目的と前提条件 {#goal-and-prerequisites}

ホストの記述子と生成イベント契約を使って自動化を実装します。非公開プロファイルで作業し、意図してテストするまで外部処理を無効にします。[イベント正規化](events.md) と[制御メタデータ](control-api.md) を理解してください。Vue エディターは Rust が所有する実行を設定します。

## 最小のルールを作る {#build-a-minimal-rule}

振る舞いアクションは設定済みアクション種別を保存し、振る舞いイベントはトリガー、フィルター、クールダウン、アクション ID を選びます。ワークフローグラフはバージョン付きノードとポートを構成します。組み込み記述子はコア、プラグイン記述子はインストール済みマニフェストから取得します。別のアクションカタログをエディターに固定しないでください。

1. `src/automation/contracts/generated/event-registry.generated.ts` から実際に送出するトリガーを選びます。
2. サンプルを見て `event.user.uniqueId` や `event.data.giftName` など実在するドット区切りフィールドを選びます。
3. ログか無効なアクションから始め、無害化したイベントでフィルターとクールダウンを試します。
4. 担当のネイティブコーディネーターを通して保存し、必要に応じて編集候補と確認済みバージョンを保持します。

```sh
bun scripts/task.ts contracts
bun scripts/task.ts contracts --check
bun scripts/task.ts test bun automation
```

[生成契約ディレクトリ](https://github.com/nglmercer/TikTools-app/tree/remake/src/automation/contracts/generated) がフロントエンドのイベント型、レジストリ情報、サンプルを所有します。フィールドを公開する際は Rust 契約を拡張して同時に再生成します。

## スクリプトの境界 {#script-boundary}

`core.code` とワークフローのスクリプトノードは制限付き QuickJS で動きます。JSON の `event`、`inputs`、および inputs の別名 `data` を受け取ります。Node ローダー、ファイルシステム、ネットワーク、WebView、DB オブジェクトはありません。変換は決定的かつ JSON 直列化可能にし、HTTP、ポイント、音声、プラグイン処理は設定済みホストアクションを使います。ソース・結果の上限と実行予算はブラウザーのタイムアウトではなくホストが管理します。

## 期待される結果と提供状況 {#expected-result-and-status}

保存したルールはコアランタイムと機能ブローカー経由で実行されます。プロセッサーは生フィールドを置換せず `event.intel` に証拠を加えられます。自動化の変更はローカル確認用で、アカウント全体の設定同期ではありません。確認付き変数はネイティブ実装済みです。オーバーレイの変更は別の未マージ作業です。

## 問題が起きたとき {#recovery}

競合では候補を失わず現行レコードと比較します。リモート下書きは所有者確認と権限移譲まで無効のままです。プレビューやマニフェスト確認だけで実行許可は得られません。外部処理の結果不明時は、保持した処理の調査・復旧フローを使います。UI 履歴の再表示で自動化を再実行してはいけません。管理された取り込みは[テンプレート](templates.md)、境界ごとの証拠は[テスト](testing.md) を参照します。

## 運用の詳細 {#operational-details}

以下のグラフはカタログ v1 の trigger → transform → log ポートを使い、無効です。`docs-graph.json` として保存し `automation.nodes.list` で版・ポートを確認します。`workflow save --record` が保存し、`workflow get` とメタデータ付き有効・無効操作で保存状態を確認します。直前の読み取り版を使います。保存・有効化だけでは全 build の完全なグラフ実行器・視覚エディターを証明しません。この checkout の通常 runtime は behavior Events/Actions を実行します。無害な実行確認は[テンプレート](templates.md#operational-details)のログ規則を確認して両レコードを意図的に有効にし、匿名化 chat を試します。グラフ保存は LIVE 実行の証明ではありません。

処理器は `events.enrich` を持つ TextIntel を導入・有効化し、Processors の状態・指標とサンプルを確認します。実際の `event.intel.providers.textintel.comment.moderation.blocked` を見てから is-true フィルターを `core.log` へ結びます。フィールド不在は false の証拠ではありません。回路開放・飽和では処理器を省略します。サンプル試験は LIVE 全体の受入試験ではありません。

所有者処理が不明なら `automation.library.get`・`automation.definition.get`・`automation.adoption.get`・`automation.approval.get` を使い分けます。pending.commandId・host・候補を保持し、`*.resolve` で元操作を解決します。unknown の場合は現状態を調べてから元 ID と明示了承で `*.review_unknown` を使います。引き継ぎは権限・依存の再レビューが必要で、inspect・preview だけでは不足します。`automation.effects.get {effectId}` は保持済み外部効果を調べるだけで送出を許可せず、盲目的な再試行も提供しません。不明結果を残して運用者が照合します。

```json
{"schemaVersion":1,"id":"docs-example","name":"Docs","enabled":false,"nodes":[{"id":"start","type":"trigger.event","version":1,"position":{"x":0,"y":0},"config":{"eventType":"tiktok.chat"}},{"id":"text","type":"transform.template","version":1,"position":{"x":200,"y":0},"config":{"template":"docs-example"}},{"id":"log","type":"action.log","version":1,"position":{"x":400,"y":0},"config":{"message":"docs-example"}}],"edges":[{"id":"a","kind":"flow","source":"start","sourcePort":"flow","target":"text","targetPort":"flow"},{"id":"b","kind":"flow","source":"text","sourcePort":"flow","target":"log","targetPort":"flow"}]}
```

```sh
tiktools api call automation.nodes.list '{}' --json
tiktools workflow save --record "$(cat docs-graph.json)"
tiktools api call workflows.get '{"id":"docs-example"}' --include-meta --json
tiktools api call workflows.enable '{"id":"docs-example"}' --expected-revision REVISION --dry-run --json
tiktools api call automation.library.get '{}' --json
tiktools api call automation.adoption.get '{}' --json
```
