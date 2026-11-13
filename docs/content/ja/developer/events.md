---
title: "イベントの正規化・バッチ・欠落"
docId: "developer.events"
translationKey: "developer.events"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "af4dd266875a9cdd3b1166b1791ab80fd20f1a4d0b72158aaa35b385a0fcdf55"
translationRevision: "af4dd266875a9cdd3b1166b1791ab80fd20f1a4d0b72158aaa35b385a0fcdf55"
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

再接続時の重複は表示配信や対応する再送保護付き計上を二重にしません。待機列、カウンター、履歴、再送キャッシュには上限があります。無制限アーカイブや普遍的な厳密1回保証ではありません。容量不足や永続化不能は健全性に明示的な影響を与えます。

## 復旧と検証 {#recover-and-verify}

無害化したギフト・エモートのフィクスチャーで、最終コンボ、ID 不足、別ルームの識別、再接続の欠落を検査します。古い信頼性報告からハッシュをコピーせず、現在の `Cargo.toml` の署名依存リビジョンを確認します。未知の配信元メソッドは、正規化と生成契約で意図的に追加するまで未知のまま扱います。
