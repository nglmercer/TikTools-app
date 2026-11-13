---
title: "Termux とヘッドレスの対応範囲"
docId: "user.start.termux"
translationKey: "user.start.termux"
locale: "ja"
audience: "user"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "12f13cdca1cf24f7de30d83fa8f3844f9d1beee6c910c4deb9be817acb89a009"
translationRevision: "12f13cdca1cf24f7de30d83fa8f3844f9d1beee6c910c4deb9be817acb89a009"
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
