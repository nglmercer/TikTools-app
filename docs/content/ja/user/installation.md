---
title: "Windows または Linux にインストールする"
docId: "user.start.installation"
translationKey: "user.start.installation"
locale: "ja"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "ed9e7c1bcb88d991ed3d8ea8a399aa5b5e9ca8cfa58de580f8ca757344d26ca2"
translationRevision: "1de85ad61053bb9109fd85b0ece0b2b925ec7745ebf6a7617d143ac89d37160b"
order: 20
---

# Windows または Linux にインストールする {#install-windows-or-linux}

## 目的と前提条件 {#goal-and-prerequisites}

コンパイラーを設定せずに、公開済みのデスクトップパッケージをインストールします。プロジェクトの [GitHub リリース](https://github.com/nglmercer/TikTools-app/releases) から取得し、使用する OS の手順に従ってください。

<DocRequirements title="始める前に">

- [動作要件](requirements.md) を確認し、使用する PC に対応したパッケージを選びます。
- アプリを置き換える前に、起動中の TikTools を終了します。
- リリースノートを読み、ダウンロードの検証に使う `SHA256SUMS.txt` を用意します。
- 別のバージョンを試す前に、アプリを停止してデータディレクトリをバックアップします。

</DocRequirements>

## パッケージを選ぶ {#choose-a-package}

<DocTabs label="使用する OS を選ぶ">

<template #windows>

ユーザーごとに NSIS でインストールする場合は、x86_64 の **`installer.exe`** を選びます。デスクトップアプリには **WebView2** が必要です。インストーラーに求められたら導入してください。

可搬型ディレクトリとして使う場合は **ZIP** パッケージを選びます。アーカイブ全体を展開し、実行ファイル、`web/`、その他の同梱リソースを一緒に保ってください。

[Windows の手順に進む](#windows)

</template>

<template #linux>

互換 Linux デスクトップで実行する可搬型イメージには **`installer.AppImage`**、Debian/Ubuntu のパッケージマネージャーで導入する場合は **`installer.deb`**、実行ファイルとリソースを含む可搬型ディレクトリには **TAR.GZ** を選びます。

AppImage には実行権限が必要です。DEB はパッケージマネージャーで実行時依存関係を解決します。TAR.GZ はディレクトリ全体を展開し、同梱リソースを一緒に保ってください。

[Linux の手順に進む](#linux)

</template>

</DocTabs>

安定版かプレリリースかを確認します。プレリリースは検証用候補で、本番互換性を自動的に保証するものではありません。既存環境を置き換える前に、リリースの `SHA256SUMS.txt` でダウンロードしたバイト列を確認します。

## Windows {#windows}

<DocSteps>

1. x86_64 インストーラーを取得し、チェックサムを確認します。
2. TikTools を使うユーザーで実行し、求められたら WebView2 をインストールします。
3. TikTools を起動し、**設定 → 一般** で表示言語を選びます。
4. 可搬型 ZIP はディレクトリ全体を展開します。`web/` などの同梱リソースを実行ファイルの隣に保ち、コピーした EXE だけを実行しないでください。

</DocSteps>

## Linux {#linux}

<DocSteps>

1. 対応する AppImage、DEB、可搬型アーカイブを取得し、チェックサムを確認します。
2. AppImage はファイルマネージャーで実行を許可するか、取得ファイルに `chmod +x` を実行してから開きます。
3. DEB は実行時依存関係が解決されるよう、システムのパッケージマネージャーでインストールします。
4. TAR.GZ は全体を展開し、同梱リソースを保ったままデスクトップ実行ファイルを起動します。

</DocSteps>

## 期待される結果 {#expected-result}

TikTools がオフライン状態で開きます。データはインストール先とは別で、通常 Windows では `%LOCALAPPDATA%/TikTools`、Linux では `~/.local/share/TikTools` に保存されます。

## 問題が起きたとき {#recovery}

ウィンドウが出る前に失敗したら、アカウントや移行設定を変える前に診断と不足ライブラリのエラーを確認します。別バージョンを試す前に、アプリを停止してデータディレクトリをバックアップします。古い実行ファイルの再インストールでは DB 移行は戻りません。[最初の LIVE](first-live.md) または[更新](updates.md) に進みます。

## 運用の詳細 {#operational-details}

ウィンドウが開かない場合はネイティブの `tiktools.log`（コントローラーモードは `controller.log`）を確認します。Windows は `%LOCALAPPDATA%/TikTools/logs`、Linux は `${XDG_DATA_HOME:-~/.local/share}/TikTools/logs` ですが、`TIKTOOLS_HOME` または `TIKTOOLS_LOG_DIR` で変わります。ファイル作成前にログ初期化が失敗する場合もあります。端末のエラーを残し、共有前に[診断の保持とプライバシー](preferences/troubleshooting.md#operational-details)を確認してください。
