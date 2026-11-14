---
title: "性能と処理量の上限"
docId: "developer.performance"
translationKey: "developer.performance"
locale: "ja"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "04bc0a89b86724872adae3ab8ddc8e29f5517dad14d51bc103d917582cf289ae"
translationRevision: "ef2e12b148f544f8d9e13e814a3be66aa2a81d6100477f8ef740e9812dbe4dd3"
order: 110
---

# 性能と処理量の上限 {#performance-and-bounded-work}

## 目的と前提条件 {#goal-and-prerequisites}

イベント計上と終了処理を維持しつつ、具体的なボトルネックを測ります。固定ビルド、決定的入力、分離プロファイルで始めます。フィード高速化と永続化の信頼性を混同しないよう[イベント](events.md) を読みます。過去のベンチマークは指定ビルドと端末の観測で、現行製品の保証ではありません。

## 適切な境界を測る {#measure-the-right-boundary}

デスクトップの outbox は欠落を許す表示と信頼性が必要な応答を分離します。スナップショットはまとめられますが、信頼側のあふれは明確に失敗させます。LIVE 取り込み、プラグイン処理、プロセッサー呼び出し、自動化受付はそれぞれ上限を持ちます。1つの待機列を増やすと、次段へメモリー負荷や遅延が移る場合があります。

フロントエンドは開発時、または `VITE_TIKTOOLS_DIAGNOSTICS=1` ビルドで集計診断を公開します。描画行数、フレーム間隔の百分位、保留中の制御呼び出し、画像、所有タイマーを含みます。WebView により JavaScript ヒープ情報は得られない場合があります。非表示ページとホスト可視性は表示の関心を制御し、業務計上は変えません。

## 対象を絞って比較する {#run-a-focused-comparison}

1. ビルドリビジョン、環境、イベント構成、画像の有無、表示タブ、時間を記録します。
2. 同じフィクスチャーと計測期間で基準を取ります。
3. 境界を1つ変え、処理量、p95/p99 遅延、待機列の破棄、欠落、保持メモリー、終了時の排出を比較します。
4. 遅いプラグイン・受信側と飽和した待機列も含めます。アイドル時の測定ではバックプレッシャーを検査できません。

```sh
bun scripts/task.ts test web live-feed-state
bun scripts/task.ts rust test -p tiktools-core --features persistence,native-tiktok --test live_throughput --locked
```

ネイティブ測定の前に機能要件を確認します。通常の対象単体検査に実際の TikTok 接続は不要です。実装の根拠として `crates/tiktools-core/tests/live_throughput.rs` とデスクトップ outbox のテストを使います。

## 期待される結果と提供状況 {#expected-result-and-status}

上限付きの投影と受付経路、カウンター、欠落報告は実装されています。[現在の順序付き書き込み器](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/services/points/queue.rs) はメモリー変更前に飽和書き込みを拒否し、同期書き込みへの代替を禁止します。過去の性能報告は既に置き換わった順序不具合を記述します。UI メモリーの上限だけから完全な正しさを推測せず、現行実装・テストでキャッシュ上限と復旧を評価します。

## 問題が起きたとき {#recover}

フレームが改善しても計上の健全性が悪化したら境界の変更を戻し、書き込み・受付結果を調べます。非表示タブのタイマー増加は[ライフサイクルの後始末](lifecycle.md) を確認します。生入力数、受付数、損失の分類を報告に残します。欠落から表示は復旧できても、拒否・結果不明になった実処理は直せません。
