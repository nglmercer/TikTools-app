---
title: "Termux とヘッドレスの対応範囲"
docId: "user.start.termux"
translationKey: "user.start.termux"
locale: "ja"
audience: "user"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "40a55076e0cff0230664728c6091cbec0b41649f9e69f50da50c741de337c2cb"
translationRevision: "45fd5544bf78f19128f39399b6a7c4519ca60061b417333072f7d5a102987b57"
order: 50
---

# Termux とヘッドレスの対応範囲 {#termux-and-headless-scope}

## 目的と前提条件 {#goal-and-prerequisites}

デスクトップ WebView なしで使える範囲を理解します。これは高度な用途で、Windows/Linux インストーラーの Android 版ではありません。プロジェクトには Termux 用ネイティブ Rust CLI とヘッドレスホストがあります。適合バイナリの入手やビルドは開発者向けセットアップで扱います。

## ホストを1つ使う {#use-one-host}

1. CLI をインストールした環境で `tiktools host --ipc` を実行し、ヘッドレスホストを1つ起動します。
2. 各ターミナルでアプリケーションホームを統一します。
3. 別ターミナルから `tiktools system info`、`tiktools live status`、`tiktools api discover` でホストを確認します。
4. 文書化された終了コマンドか Ctrl-C で正常終了します。動作中に所有権ロックを消さないでください。

通常の CLI コマンドは既存ホストに接続します。`--standalone` は1回の呼び出し専用に別のコアを明示的に作るもので、起動中のホストを操作する既定の方法ではありません。

## ローカルブラウザープレビュー {#local-browser-preview}

任意の `local-web` アダプターはローカルペアリング後、Vue 画面をブラウザーへ配信できます。別のビルドとライフサイクル要件を持つ開発プレビューです。Android を対応デスクトップ WebView 対象にするものでも、遠隔の人にホストへのアクセスを与えるものでもありません。

## 期待される結果 {#expected-result}

CLI とデスクトップは操作の概念、SQLite 永続化、ネイティブイベント処理を共有します。環境依存のプラグイン、音声、ホットキー、ネイティブウィンドウ、認証情報ストアは使えない場合があります。ネイティブのリモートプロファイル永続化には、現在 Android 固有の制限があります。

## 問題が起きたとき {#recovery}

実行や Unix ソケットを妨げる場合がある Android 共有ストレージを避け、Termux の専用アプリディレクトリを使います。IPC を調べる前に同じホームを使い、ホストの稼働を確認します。Android が停止したら同じプロファイルで再起動して復旧診断を読みます。ドキュメントのデモで Android のライフサイクルを再現できるとは考えないでください。

実際のツールチェーン、機能フラグ、端末上の検証は[開発者ガイド](../developer/index.md)、デスクトップバイナリの導入は [Windows/Linux ガイド](installation.md) を参照します。

## 運用の詳細 {#operational-details}

ローカル browser adapter には index.html を含む assets と `local-web` CLI を作ります。継承パスを解除し使い捨て `TIKTOOLS_HOME` を指定、先に IPC host を起動します。同じ home の別端末で adapter を実行し、loopback 一クリック link または 6 桁一回限り code（5 分有効）で接続します。終了時は adapter と host を別々に止めます。loopback/pairing は遠隔・LAN access を提供しません。Termux は native Android toolchain と web assets が必要で、desktop Linux binary は Android binary ではありません。

```sh
bun scripts/task.ts frontend-build
bun scripts/task.ts rust build -p tiktools-cli --features local-web --locked
export TIKTOOLS_HOME=/absolute/path/disposable-home
target/debug/tiktools host --ipc
```

```sh
export TIKTOOLS_HOME=/absolute/path/disposable-home
target/debug/tiktools web --assets /absolute/path/to/TikTools/dist/web --port 4174
```
