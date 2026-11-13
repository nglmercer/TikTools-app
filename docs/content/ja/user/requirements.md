---
title: "動作要件"
docId: "user.start.requirements"
translationKey: "user.start.requirements"
locale: "ja"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "f3bc30b94983b95c7ce6ffc7fe8b3707aa175cadd2156f26f16b8b77be83ea6b"
translationRevision: "f3bc30b94983b95c7ce6ffc7fe8b3707aa175cadd2156f26f16b8b77be83ea6b"
order: 10
---

# 動作要件 {#requirements}

## 目的 {#goal}

インストール前に対応デスクトップパッケージを選び、目的の用途に必要なサービスを確認します。

## 必要なもの {#what-you-need}

- 公開デスクトップパッケージ用の Windows x86_64 または Linux x86_64 PC。現在のリリース処理はこの2対象をビルドし、macOS デスクトップ版や Android APK は公開しません。
- 正常なグラフィカルセッション。Windows は Microsoft WebView2 Runtime、Linux は互換性のある GTK/WebKitGTK 環境が必要です。DEB は実行時パッケージを依存関係として宣言し、AppImage は確認済みメディア補助機能を同梱します。
- 実際の LIVE 接続には TikTok へのネットワークアクセス。配信終了、ゲストセッションの期限切れ、配信元のレート制限は自動化を変更しても直りません。
- SQLite データ、プラグイン、ログを保存する、インストール先とは別の領域。アプリはユーザーごとのデータディレクトリを使い、実行ファイルを移動してもデータは移動しません。

Cloud アカウントはアカウント方式のカタログや共同操作に必要で、すべてのローカル編集に必要なわけではありません。OBS は配信にウィジェットを表示する場合だけ必要です。音声・TTS 連携には、別途インストールするプラグインと専用のプロバイダー・サービスが必要な場合があります。

## 環境を確認する {#check-the-environment}

1. OS と x86_64 アーキテクチャに合うパッケージを選びます。
2. Windows のインストーラーが不足を報告したら WebView2 をインストールします。Linux の不足ライブラリはディストリビューションのパッケージマネージャーで解決します。
3. TikTok 接続やプラグイン追加の前に、アプリのウィンドウが開くことを確認します。
4. [インストール](installation.md)、[最初の LIVE](first-live.md) の順に進みます。

## 期待される結果 {#expected-result}

オフラインでもアプリが開き、メインナビゲーションを使えます。任意の連携は、プラグインやホスト機能が揃うまで使えない場合があります。

## 問題が起きたとき {#recovery}

空白ウィンドウや起動直後の終了では、起動診断と正確なパッケージバージョンを記録します。AppImage と DEB は異なるインストール形式で、どちらも Android 実行ファイルではありません。その環境については [Termux とヘッドレスの対応範囲](termux.md) を参照します。ソースビルドには開発者ガイドに記載された追加要件があります。
