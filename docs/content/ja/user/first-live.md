---
title: "最初の LIVE 接続"
docId: "user.start.first-live"
translationKey: "user.start.first-live"
locale: "ja"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "fff247a8ec59baa1ebf6f4ad0019f456f1f48cfdd95ab624007bda0f7f208c96"
translationRevision: "faf4c7ffdeebe01eeeedd05ad3411d23c1235af81a238ff8ce421d25b0a4706f"
order: 30
---

# 最初の LIVE 接続 {#first-live-connection}

## 目的と前提条件 {#goal-and-prerequisites}

配信者を1人選び、イベントを受信して詳細を確認してから、自動化を有効にしましょう。実際の LIVE に接続するには、正常に動くデスクトップ版、インターネット接続、現在配信中の配信者が必要です。

<DocCallout title="本番前に操作を試す" tone="tip">

下の例は実際の操作画面と架空のデータを使います。TikTok アカウントなしで `demo_creator` に接続できます。デモで成功しても、実際の TikTok セッションが有効であるとは限りません。Cloud へのサインインは別の接続で、TikTok を認証するものではありません。

</DocCallout>

## 接続する {#connect}

<DocSteps>

1. **配信者を指定します。** **接続**、または下部ステータスバーの **接続** を開き、クリエイターのハンドルを入力します。先頭の `@` は省略できます。表示名ではなく識別子を使ってください。
2. **接続オプションを選びます。** ゲスト接続が可能な場合は **セッションCookieを使用** を無効にします。許可された自分のセッションが必要なルームでは有効にし、専用のマスク欄だけに Cookie を入力します。最初の直接接続では **監視して自動接続** を無効のままにします。
3. **接続状態を確認します。** **接続** を押し、状態が確定するまで待ちます。下の例で `demo_creator` に接続し、接続インジケーターの変化を確認してください。

</DocSteps>

<DemoFrame scenario="connection" locale="ja" panel="connection" title="LIVE 接続パネルを操作する" />

## 期待される結果 {#expected-result}

状態が接続済みに変わります。**ライブフィード** を開き、実際のイベントを待つか、問題のないチャットを依頼します。配信者と時刻を確認し、イベントを選んで詳細を表示します。視聴者名かアバターを選ぶと、記録されているプロフィールを確認できます。

<DemoFrame scenario="feed" locale="ja" panel="feed" title="架空の LIVE イベントを確認する" />

静かなルームでは新しいイベントが届かないことも正常です。ポイントは有効なポイントルール、自動化の結果は設定したアクションとイベントに依存します。イベントの受信だけで、それらの機能が自動的に有効になるわけではありません。

## 監視と再接続 {#monitoring-and-reconnecting}

直接接続に成功した後、**監視して自動接続** で配信開始を待てます。状態には待機、確認中、接続処理が区別されます。固定された配信者欄を編集するには監視を停止します。同じ配信者への再接続では現在のフィードを保持し、配信者を変えると別のフィード対象になります。

## 問題が起きたとき {#recovery}

オフラインなら配信中か確認してから再試行します。認証やレート制限のエラーでは **診断** を読み、表示された再試行条件に従います。連打しても、配信元の期限切れセッションは直りません。

<DocCallout title="セッション Cookie を公開しない" tone="warning">

Cookie が必要なら、新しい許可済みセッションを取得します。値を画像、ログ、問い合わせに含めないでください。Cookie はメモリーに保持され、Cloud 用の再利用可能な認証情報ではありません。別の配信者を試す前に切断してください。

</DocCallout>
