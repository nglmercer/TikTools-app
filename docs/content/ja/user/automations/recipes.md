---
title: "実用的な自動化レシピを応用する"
docId: "user.automations.recipes"
translationKey: "user.automations.recipes"
locale: "ja"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "43a103ef68e43d691eec2214ffefb5e2e0e6b663afa9f4e63b5dac0ccec162ea"
translationRevision: "7cf62155024ff018d9d7546a36872ecab40632a41167439bbaaeb853272f69e5"
order: 30
---

# 実用的な自動化レシピを応用する {#adapt-practical-automation-recipes}

## 目的と前提条件 {#goal-and-prerequisites}

正常に動く[ログ記録ルール](first-rule.md) を、条件を絞った反応へ発展させます。アクティブなプロファイルと、実際に確認したイベントのフィールドが必要です。まず `core.log` を使い、一致条件と頻度を確認してから別の処理に置き換えます。

## チャットコマンドに一致させる {#match-a-chat-command}

`tiktok.chat` イベントを作り、`event.data.comment` が `!こんにちは` に等しい (`eq`) 条件を設定します。ログは次のようにします。

```text
こんにちは {{ event.user.uniqueId }}
```

ユーザー別クールダウンを `30` 秒にすると、1人が連続して実行することを防ぎ、別の視聴者には独立した機会を残せます。コマンドには完全一致が便利です。「含む」では前後に別の文言があるメッセージも一致するため、意図に合わせて選んでください。

## 条件を満たすギフトを記録する {#record-a-qualifying-gift}

`tiktok.gift` イベントを作り、`event.data.diamondCount` が `10` 以上 (`gte`) の条件を設定します。ログは次のとおりです。

```text
{{ event.user.uniqueId }} からのギフト: {{ event.data.giftName }}
```

最初に実際のギフト詳細を確認します。ダイヤモンドの価値、連続回数、完了フラグは別々のフィールドです。完了時だけ反応させるなら、受信ペイロードに `event.data.repeatEnd` があることを確認し、真偽値 true の条件を使います。名前のフィルターだけで、コンボ完了ごとに1回実行されるとは判断できません。

## 共有する反応の頻度を制限する {#bound-a-shared-reaction}

`tiktok.follow` イベントに全体のクールダウン `10` 秒を設定し、短いログアクションを関連付けます。その間に2人がフォローすると、後の反応が抑制されることがあります。視聴者ごとに機会が必要ならユーザー別の範囲を使います。

複数のアクションでは一覧を順に並べ、全手順が必要なら全アクション実行モードを維持します。ランダムモードは1つを選ぶ方式で、全手順をランダム順に実行する方式ではありません。後続手順を待たせる場合だけ `core.delay` を追加します。

## 期待される結果と提供状況 {#expected-result-and-status}

これらのトリガー、フィールドフィルター、クールダウンはローカルエンジンで利用できます。有効状態とクールダウンの制約の下で、受け付けた一致イベントに対応する実行結果が得られます。プロバイダー固有の動作は実際の LIVE で確認する必要があります。

## 問題が起きたとき {#recovery}

失敗したらログ1つに戻し、ペイロードを確認して一時的に条件を簡単にします。調査中も混雑した配信の制限を解除しないでください。音声、ポイント、HTTP、プラグインのテストでは実際の処理が動きます。追加前に[音声設定](../plugins/audio.md)、[変数](variables.md)、[実行時の診断](../preferences/troubleshooting.md) を確認します。
