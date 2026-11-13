---
title: "プラグインのランタイムと信頼の制限"
docId: "developer.plugins.runtimes"
translationKey: "developer.plugins.runtimes"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "de1679ef655bf91d46a7247f3d4e36bd6140c30476f0739c2cae872588e80aaa"
translationRevision: "de1679ef655bf91d46a7247f3d4e36bd6140c30476f0739c2cae872588e80aaa"
order: 260
---

# プラグインのランタイムと信頼の制限 {#plugin-runtimes-and-trust-limits}

## 目的と前提条件 {#goal-and-prerequisites}

依存と信頼がホストに合う実行形式を選びます。[概要](index.md) を読み、実際のビルドに対してマニフェストを検証し、読み込み前に対象固有の制限を確認します。プロトコル互換性、ネイティブ ABI 互換性、権限付与は独立した検査です。

## 実装済みランタイムを選ぶ {#choose-an-implemented-runtime}

| ランタイム | 実行 | 信頼と可搬性 |
| --- | --- | --- |
| `native` | `libloading` C ABI | 信頼済みのプロセス内コード、対象固有 |
| `process` | 独立実行ファイル、フレーム化 stdin/stdout | クラッシュ分離、通常ユーザーの OS 権限 |
| `napi-vm` | `rquickjs` 経由で事前コンパイル JS を QuickJS 実行 | ホスト処理は既定で拒否。任意のネイティブアドオンで信頼条件が変わる |
| `declarative` | 検証済み連携記述子 | ゲストプログラムなし。ホストブローカーで処理 |
| `wasm` | 任意の実行枠 | 通常ビルドでは汎用エンジン無効 |

ネイティブ ABI v1 は小さな C テーブルを通して直列化バイト列を交換し、Rust コンテナー、トレイトオブジェクト、Future は渡しません。ライブラリは終了まで読み込んだままです。process のフレームは little-endian u32 の長さと JSON バイト列で、改行区切り制御 RPC とは異なります。診断出力をフレーム用 stdout に混ぜないでください。

## QuickJS 互換性を理解する {#understand-quickjs-compatibility}

`napi-vm` 名は維持しますが、独立した純 Rust インタープリターという旧説明は過去のものです。ゲストにあるのは文書化された互換グローバルで、完全な Node 環境ではありません。`napi_get_uv_event_loop` は非対応です。ネイティブ `.node` は意図的に信頼し、許可リストと SHA-256 検証を使うプロセス内経路です。JavaScript ランタイムの名前で安全になるわけではありません。

```sh
bun scripts/task.ts rust test -p tiktools-plugin-loader --lib --locked
bun scripts/task.ts test bun packages
```

完全なネイティブアドオンの検証に不足ツールが必要なら、対象ローダーテストを使います。

## 期待される結果と提供状況 {#expected-result-and-status}

マニフェスト解析は読み込み前に非対応のプロトコル・ABI を拒否します。Android は可搬 QuickJS と有効な Android/Bionic process を許可し、動的ネイティブライブラリと `.node` を拒否します。Linux/glibc 成果物は Android 用 process プラグインではありません。

## 問題が起きたとき {#recovery}

ネイティブライブラリ置換後は非公開ホストを再起動します。起動がタイムアウトして不明になったら、管理側がワーカー ID を保持して置換を禁止する場合があります。終了成功を宣言して迂回しません。イベントバッファーは有限で遅れが報告され、起動時の送出は永続待機列ではありません。[ローダー実装](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-plugin-loader/src) が現行エンジンとライフサイクルの正本です。
