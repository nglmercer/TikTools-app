---
title: "テストと証拠の範囲"
docId: "developer.testing"
translationKey: "developer.testing"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "455bcdbf94d1bff28bd50dcb993cdff59dd000214e25c49bb194c9b3124e8e59"
translationRevision: "455bcdbf94d1bff28bd50dcb993cdff59dd000214e25c49bb194c9b3124e8e59"
order: 210
---

# テストと証拠の範囲 {#tests-and-evidence-boundaries}

## 目的と前提条件 {#goal-and-prerequisites}

変更した動作を確認できる検査を選びます。Bun 1.4.4、ロックファイル通りのインストール、影響するネイティブ境界にだけ Rust を使います。本番ホスト、認証情報、アカウントからテストを分離します。ブラウザーフィクスチャーの成功はその境界には有用ですが、下層の全連携の証明にはなりません。

## 対象を絞ったテストを選ぶ {#choose-a-focused-suite}

```sh
bun scripts/task.ts test
bun scripts/task.ts test web control-client
bun scripts/task.ts test bun automation
bun scripts/task.ts test rust tiktools-core batched_likes --features persistence,native-tiktok
bun scripts/task.ts rust test -p tiktools-client --test parity --locked
bun scripts/task.ts contracts --check
```

既定の高速セットは Bun 単体テストと web/widget の Vitest を組み合わせます。対象ランナーは引数を検証し、Rust フィルター不足を拒否して、一致ゼロの成功を受け入れません。専用構文を超える Cargo 対象フラグは `rust` 引き渡しを使います。

## 実際の境界を検証する {#verify-the-actual-boundary}

Vue 部品テストは共有の偽制御クライアントを使います。主要 Playwright テストは移動前に入れた偽 `window.ipc` ホストで同じ SPA を操作します。未知の RPC は明確に失敗し、console・ページエラーも検査します。動くのは Wry ではなく Chromium です。プラグイン UI のブラウザーテストは外部プラグインのリポジトリにあります。ウィジェットゲートウェイのテストは共有通信・クライアントとビルド済みアセットを検証します。

アセットのビルド後、許可されたブラウザー作業には `bun scripts/task.ts test e2e` を使います。ブラウザーを起動するため、文書編集や画面なしのソース確認とは意図的に分かれています。ネイティブシェル、OBS、端末、OS 認証情報ストア、更新置換、2 PC テストは実環境と明示的証拠が必要です。

## 実行時間と CI コスト {#execution-time-and-ci-cost}

ローカルではファイル・名前フィルターか `bun scripts/task.ts test --changed` から始めます。完全なフロントエンド実行もファイルを分離します。Vitest の `vmThreads` はワーカーごとに DOM ランタイムを再利用しますが、ファイル間のアプリ状態は共有しません。

CI は push・PR の完全な範囲を比較します。確認済み文書パスはネイティブジョブを省き、Vue/demo 変更はフロントエンド条件を実行します。ネイティブソース、ビルド基盤、依存マニフェスト、不明パス、比較不能では全条件を維持します。検証済み選択方針は `scripts/ci-scope.mjs` です。手動 CI は `frontend`、`native`、`full`、再利用呼び出しは既定で完全検証です。最終 CI は選択ジョブの失敗・キャンセルを拒否します。

Rust キャッシュは役割、OS、ツールチェーン、ロックファイル、ソースリビジョンで分かれます。CI は行テーブルのデバッグ情報を残し、増分セッションを無効化して容量を抑えます。SQLite・プロファイルの共有リソースが独立するまで直列テストを維持し、時間短縮のため保護を外しません。

サイトの処理は手動なので、文書の編集・push だけでは別のリモートビルドを開始しません。リリース復旧も手動で、既定は Linux 検査のみ・公開なしです。公開には両環境の条件と明示的な公開選択が必要です。Markdown 確認のためだけに有料ランナーの実行を開始・反復しないでください。

## 期待される結果と提供状況 {#expected-result-and-status}

変更報告は成功したテストと範囲を明記します。実装変更は frontend lint・型・build と必須 CI、文書は `bun scripts/task.ts docs check` と `docs test` を使います。新しいツールをダウンロードさせるためだけに依存を変えません。

## 問題が起きたとき {#recovery}

一致ゼロなら成功報告せずフィルターを修正します。必須 RPC が足りなければ未知呼び出しを成功扱いせず、偽ホストの明示契約を拡張します。不確実・省略した手動受け入れは明示します。現行構文は[テストランナー](https://github.com/nglmercer/TikTools-app/blob/remake/scripts/lib/test-runner.ts)、再現可能な負荷証拠は[性能](performance.md) を参照します。
