---
title: "イベントの正規化・バッチ・欠落"
docId: "developer.events"
translationKey: "developer.events"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "caded5529a62e826a91d2deb86936c29c709ac713c2a0cd021f467b1dfa2b6e7"
translationRevision: "45983681dff981890e7bff9bddc6d0f5d341c32f530b2b7f4a59c18ecc87bd4c"
order: 100
---

# イベントの正規化・バッチ・欠落 {#event-normalization-batches-and-gaps}

## 目的と前提条件 {#goal-and-prerequisites}

二重計上や表示復旧と実行の混同を避けて LIVE イベントを利用します。[永続化](storage.md) を理解し、実アカウントの記録ではなく無害化したフィクスチャーを使います。`tiktools-tiktok` は固定された署名・デコード依存関係を包み、安定した TikTools の値を公開します。コアは protobuf オブジェクトをアプリ API として公開しません。

## 実装された流れを追う {#follow-the-implemented-pipeline}

取り込みは非同期処理の前に元の配信者・ルーム接続の文脈を保持します。正規化はチャット、ギフト、いいね、交流、ライフサイクルの値を生成します。コアは元のイベントを公開し、計上・実処理を別管理します。プロセッサーは `event.intel` に制限付き派生証拠を付けられ、失敗時も生イベントを劣化診断付きで保持します。生成自動化契約は実際に送出されるフィールドだけを記述します。

バックエンドの再送識別には配信者、ルーム、メソッド、型、ゼロ以外のメッセージ ID を使います。ID がなければ再送と確定できません。ギフトコンボの計上は送信者・ギフト・グループと累積最大値を使い、最終パケットで同量を二重付与しません。不明な価格はゼロです。集計遷移は `previous < threshold <= total` による閾値越えを支援できますが、公開する元イベントの代わりにはなりません。

## 受信側を実装する {#implement-a-consumer}

1. `src/automation/contracts/generated/` の型か、その境界用のウィジェットドメイン包みのパーサーを使います。
2. 直列化を通して配信カーソルを保持し、リアルタイムと補完をつなぐ際に重複を除きます。
3. イベントごとに再描画せず、順序付きバッチを処理します。
4. 欠落・過負荷を明示します。制限付きページや新しいスナップショットで表示を復旧し、ポイントやアクションを再実行しません。

```sh
bun scripts/task.ts contracts --check
bun scripts/task.ts test web live-feed-state
bun scripts/task.ts test bun automation
```

[Rust 契約ソース](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-core/src/contracts) がレジストリ、スキーマ、サンプルを生成します。完全なフィールド表を手で複製しないでください。

## 期待される結果と提供状況 {#expected-result-and-status}

ID 付きイベントが現在の有限 replay store に残る間だけ、再接続の重複を排除します。待機列、カウンター、履歴、再送キャッシュには上限があります。無制限アーカイブや普遍的な厳密1回保証ではありません。容量不足や永続化不能は健全性に明示的な影響を与えます。

## 復旧と検証 {#recover-and-verify}

無害化したギフト・エモートのフィクスチャーで、最終コンボ、ID 不足、別ルームの識別、再接続の欠落を検査します。古い信頼性報告からハッシュをコピーせず、現在の `Cargo.toml` の署名依存リビジョンを確認します。未知の配信元メソッドは、正規化と生成契約で意図的に追加するまで未知のまま扱います。

## 運用の詳細 {#operational-details}

現在の既定値（2026-11-13）は実装上の制限で、SLA ではありません。LIVE 入口は 2,048 件、副作用キューは 128 件で、バイト上限ではありません。ライフサイクルイベントは空きを待ちます。ギフト・フォロー・シェアは最大 1 秒待ってから欠落を記録して破棄し、チャット・いいね・参加・その他は飽和時に即時破棄します。`droppedCritical`、いいねのバッチ・単位、副作用の欠落数を確認します。入力の欠落では副作用自体が未実行の場合があります。Backfill は表示だけを回復し、ポイントやアクションを補完しません。

重複排除はプロセス内で最大 4,096 ID を 300 秒保持し、そのストアが存続する間だけ再接続をまたぎます。期限切れ、容量による削除、ID 不在、再起動では保護されません。再接続の重複に対する保護もこの範囲内です。

`ui.feed.backfill` は `{version:1,after:0,limit:64}` を受け取り、全体の数値カーソルと 1–512 の上限を使います。`oldestCursor`、`latestCursor`、`gap`、`truncated` を調べ、最後のイベントカーソルから続けます。`ui.feed.creator` は `{creatorId,streamId,after,limit}`、配信者別ストリームと文字列カーソル、1–64 件とバイト予算を使います。両者を混用しません。ストリーム変更・削除、未来のカーソル、入口の欠落、`CreatorFeedUnavailable` では正規の表示状態を再取得し、履歴不足を明示します。副作用を再構築してはいけません。

TikTok はセッション、ルーム、ギフトカタログ、署名 bundle、socket の順に初期化します。設定は `TIKTOOLS_TIKTOK_SIGNING_BUNDLE`、`TIKTOOLS_TIKTOK_BUNDLE_CACHE`、`TIKTOOLS_TIKTOK_BUNDLE_URL` です。既定は 5 回、初期待機 2 秒、上限 30 秒。HTTP 429 は Retry-After または 30–300 秒へ増加する最小待機とジッターを使い、制限時の再試行は最大 2 回で、中止・枯渇後も待機を保持します。他は一時的な通信障害だけを再試行します。`Reconnecting { attempt, delay_ms }` と失敗段階を確認し、Cookie や非公開署名情報は共有しません。

```sh
tiktools api call system.health '{}' --json
```
