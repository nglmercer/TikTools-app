---
title: "LIVE 接続を管理する"
docId: "user.live.connections"
translationKey: "user.live.connections"
locale: "ja"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "e0d1fec65d919e7ca9946024142bbbe1bca8b3851f7e66246dcaddd101cd5215"
translationRevision: "1d2fabd8b851574030b71db38057adacd4a70f7a8acf5866f3d983ee5cf518e9"
order: 10
---

# LIVE 接続を管理する {#manage-live-connections}

## 目的と前提条件 {#goal-and-prerequisites}

特定の配信者への接続と再接続を理解し、プラグインサーバーを独立して設定します。正常な[デスクトップ環境](../installation.md)、ネットワーク、配信者のハンドルが必要です。OS の問題を診断する前に[動作要件](../requirements.md) を確認します。

## 接続内容を確認して操作する {#connect-deliberately}

1. **接続** でハンドルを入力します。`@` の有無は問いません。表示名とは異なる識別子です。
2. **セッションCookieを使用** を無効にして始めます。TikTok が許可済みセッションを求めたら有効にし、マスク欄だけに Cookie を入力します。Cookie はメモリー内に留まります。診断や Cloud 設定に貼り付けないでください。
3. 直接試す場合は **監視して自動接続** を無効にして **接続** を選びます。再試行前に状態とエラーを読みます。
4. [ライブフィード](feed.md) で正しい配信者と新しいイベントを確認します。接続済みでも静かなルームなら動きがない場合があります。
5. 同じ配信者には **再接続**、配信者を変える前には **切断** を使います。処理中の試行は **キャンセル** で止めます。

直接接続に成功した後、監視を有効にして配信開始を待てます。待機・確認中は接続済みとは異なります。固定されたハンドルの編集前に監視を停止します。最近の配信者を選ぶ操作は、入力を埋めるだけでなく接続も開始します。

エクスプローラーには検出したルームが表示されますが、古くなることがあります。利用可能と判断する前に更新してください。下部パネルのプラグインサーバー接続は各サービスの設定で、プラグイン接続済みでも TikTok や Cloud の認証にはなりません。

## 期待される結果と提供状況 {#expected-result-and-status}

直接接続、監視、プラグイン接続の操作を利用できます。同じ配信者への再接続はフィードの対象を保持し、配信者の変更は対象を切り替えます。実際の TikTok アクセスは配信元のルームとセッションに依存し、ローカルテストの成功では保証されません。

## 問題が起きたとき {#recovery}

オフラインのルームでは配信中か確認します。セッションやレート制限のエラーでは示された再試行条件に従います。再接続の連打では認証は更新されません。[トラブルシューティング](../preferences/troubleshooting.md) に従い、認証情報を出さずに失敗した要素と時刻を記録します。

実際の設定に触れる前にインタラクティブに試せます:

<DemoFrame scenario="connection" locale="ja" panel="connection" title="LIVE接続パネルを試す" />

## コマンドライン操作 {#command-line-operations}

このガイドのすべての操作はCLIからも利用できます——オフライン、スクリプト可能、エージェント対応。使用法の行はパーサー構文であり、すべての言語で英語のままです。

<!--@include: ../../../../.generated/cli-domain-live.ja.md-->

規約、JSON出力、破壊的操作のマーカーについては[完全なコマンドリファレンス](../../developer/cli.md)を参照してください。

<TourFrame id="feed" locale="ja" title="インタラクティブガイド: ライブフィード" />

<TourFrame id="feedInspector" locale="ja" title="インタラクティブガイド: フィードインスペクター" />

<TourFrame id="connection" locale="ja" title="インタラクティブガイド: クリエイターに接続" />

<TourFrame id="connectionRecovery" locale="ja" title="インタラクティブガイド: 接続の復旧" />
