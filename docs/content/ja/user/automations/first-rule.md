---
title: "最初のログ記録ルールを作る"
docId: "user.automations.first-rule"
translationKey: "user.automations.first-rule"
locale: "ja"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "fc9bfc55b0fa1e4d41cdbdf75ec6c865707d53b3f8cf823871a97a0b7c9cfff2"
translationRevision: "fc9bfc55b0fa1e4d41cdbdf75ec6c865707d53b3f8cf823871a97a0b7c9cfff2"
order: 20
---

# 最初のログ記録ルールを作る {#create-your-first-logging-rule}

## 目的と前提条件 {#goal-and-prerequisites}

ローカルのログを1行出力し、イベントからアクションまでの流れを確認します。デスクトップホストが起動し、**自動化** を開ける必要があります。Cloud アカウント、音声サービス、外部エンドポイントは不要です。ログには展開後のメッセージが記録されるため、架空の内容か機密性のないチャットを使ってください。

## アクションを作る {#create-the-action}

1. 使用するプロファイルで **自動化 → アクション** を開きます。
2. **メッセージをログに記録** (`core.log`) のアクションを作り、`チャット確認` と名付けます。
3. メッセージ欄に次のテンプレートを設定し、保存してアクションを有効にします。

```text
チャット確認: {{ event.user.uniqueId }} の発言: {{ event.data.comment }}
```

これらは現在の正規化済みチャットのパスです。`event.data.comment` はメッセージ、`event.user.uniqueId` は視聴者のハンドルです。表示名には別のフィールドを使います。

## イベントを作る {#create-the-event}

1. **イベント** を開き、`チャット確認イベント` を作成して、トリガーに `tiktok.chat` を選びます。
2. `チャット確認` だけを関連付けます。通常の全アクション実行モードを維持します。
3. 最初のサンプルテストではフィルターを空にします。後で実際の通信量を制限できるよう、全体のクールダウンを `10` 秒にします。
4. 保存し、変更のプレビューが表示されたら確認します。ログのアクションだけが関連付けられた状態で **テストを実行** を選択し、返された実行結果とログを確認します。ホストがサンプルを提供する場合があり、その成功だけでは TikTok への実接続を確認できません。
5. `event.data.comment` に、等しい (`eq`)、値 `!こんにちは` の条件を追加します。再保存し、イベントを明示的に有効にします。
6. テストを許可された [LIVE](../live/connections.md) に接続します。[フィード](../live/feed.md) で新しい `!こんにちは` のチャットを確認し、クールダウン経過後に自動化の履歴を調べます。

## 期待される結果と提供状況 {#expected-result-and-status}

ローカルのログアクションとチャットトリガーを利用できます。サンプルでは読めるログが出力され、条件に合う実際のチャットでは実際のハンドルとメッセージを含む実行結果が得られます。それ以外のチャットは完全一致の条件を通過しないはずです。

## 問題が起きたとき {#recovery}

フィルター追加後にサンプルが一致しなくなった場合、サンプルのコメントが `!こんにちは` と異なる可能性があります。繰り返し実行せず、実イベントのフィールド、アクティブなプロファイル、有効状態を確認します。テストでも実際の処理が動くため、ログだけに限定すると影響を予測できます。終了時にイベントを無効にするか、[実用例](recipes.md) に進んでください。

<DemoFrame scenario="automation" locale="ja" tour="automation" title="デモ: 自動化" />
