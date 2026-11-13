---
title: "自動化の実行とスクリプトの契約"
docId: "developer.automations"
translationKey: "developer.automations"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "5552b08e589e44879d88ee299720ed1644285e42fa201261b1cf377fd163f865"
translationRevision: "5552b08e589e44879d88ee299720ed1644285e42fa201261b1cf377fd163f865"
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
