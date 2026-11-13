---
title: "Windows または Linux にインストールする"
docId: "user.start.installation"
translationKey: "user.start.installation"
locale: "ja"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "555ad595d29dac6659c260a8f2d99acc65bdf8bba49ca87d1517f527ab15469d"
translationRevision: "555ad595d29dac6659c260a8f2d99acc65bdf8bba49ca87d1517f527ab15469d"
order: 20
---

# Windows または Linux にインストールする {#install-windows-or-linux}

## 目的と前提条件 {#goal-and-prerequisites}

コンパイラーを設定せずに公開デスクトップパッケージをインストールします。[動作要件](requirements.md) を確認し、起動中の TikTools を終了して、プロジェクトの [GitHub リリース](https://github.com/nglmercer/TikTools-app/releases) からダウンロードします。

## パッケージを選ぶ {#choose-a-package}

| パッケージ | 用途 |
| --- | --- |
| Windows `installer.exe` | ユーザーごとの NSIS インストール。WebView2 が必要です。 |
| Linux `installer.AppImage` | 互換 Linux デスクトップ用の可搬型実行イメージ。 |
| Linux `installer.deb` | Debian/Ubuntu のパッケージマネージャーでインストール。 |
| Windows ZIP / Linux TAR.GZ | 実行ファイルとリソースを含む可搬型ディレクトリ。 |

安定版かプレリリースかを確認します。プレリリースは検証用候補で、本番互換性を自動的に保証するものではありません。既存環境を置き換える前に、リリースの `SHA256SUMS.txt` でダウンロードしたバイト列を確認します。

## Windows {#windows}

1. x86_64 インストーラーを取得し、チェックサムを確認します。
2. TikTools を使うユーザーで実行し、求められたら WebView2 をインストールします。
3. TikTools を起動し、**設定 → 一般** で表示言語を選びます。
4. 可搬型 ZIP はディレクトリ全体を展開します。`web/` などの同梱リソースを実行ファイルの隣に保ち、コピーした EXE だけを実行しないでください。

## Linux {#linux}

1. 対応する AppImage、DEB、可搬型アーカイブを取得し、チェックサムを確認します。
2. AppImage はファイルマネージャーで実行を許可するか、取得ファイルに `chmod +x` を実行してから開きます。
3. DEB は実行時依存関係が解決されるよう、システムのパッケージマネージャーでインストールします。
4. TAR.GZ は全体を展開し、同梱リソースを保ったままデスクトップ実行ファイルを起動します。

## 期待される結果 {#expected-result}

TikTools がオフライン状態で開きます。データはインストール先とは別で、通常 Windows では `%LOCALAPPDATA%/TikTools`、Linux では `~/.local/share/TikTools` に保存されます。

## 問題が起きたとき {#recovery}

ウィンドウが出る前に失敗したら、アカウントや移行設定を変える前に診断と不足ライブラリのエラーを確認します。別バージョンを試す前に、アプリを停止してデータディレクトリをバックアップします。古い実行ファイルの再インストールでは DB 移行は戻りません。[最初の LIVE](first-live.md) または[更新](updates.md) に進みます。
