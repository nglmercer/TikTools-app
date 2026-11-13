---
title: "リリース・パッケージ・更新機能の有効条件"
docId: "developer.releases"
translationKey: "developer.releases"
locale: "ja"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "e28f094fd59897eee4cc337e63a565849b521c329842e96dddaf891e521d0378"
translationRevision: "e65a4276e28177aa883be5e6c4988f2abc106d3116eed2c587a2b7922ea2bc94"
order: 220
---

# リリース・パッケージ・更新機能の有効条件 {#releases-packages-and-updater-gates}

## 目的と前提条件 {#goal-and-prerequisites}

確認可能なリリース成果物を準備し、検証済みダウンロードと自動インストールを区別します。対応環境のツールチェーンと保護された署名素材が必要な保守担当者向け作業です。通常の貢献者は公開やタグ作成をせず梱包・版の検査を確認できます。

## 整合した成果物を準備する {#prepare-consistent-artifacts}

`package.json`、ワークスペース `Cargo.toml`、`Cargo.lock` は同じアプリ版を使います。1つだけ編集せずタスクランナーを使います。

```sh
bun scripts/task.ts version check
bun scripts/task.ts frontend-build
bun scripts/task.ts rust build -p tiktools-desktop --release --locked
```

意図した版変更は、選んだ次版で `bun scripts/task.ts version bump v4.2.3` を使います。完全ビルドはウィジェットも生成します。梱包はネイティブ実行ファイルを `web/index.html` の隣に置き、チェックサムを公開します。インストーラー処理は NSIS、DEB、AppImage も作り、可搬型の準備内容とリソースを比較します。プラグインは別途インストールします。

## 更新の信頼を確認する {#review-updater-trust}

`tiktools-app-update` は署名付きチャンネル情報とパッケージを使います。信頼する公開鍵、フィード有効化、公開済み検証アセットは別の要件です。SHA256 一覧だけでは更新機能は有効になりません。アプリ更新鍵とプラグイン発行者鍵は分けます。

ホストはダウンロードとリダイレクトを制限し、インストール引き渡しは安全なコア・WebView 終了を待ちます。AppImage にはビルド時の `TIKTOOLS_APP_UPDATE_INSTALL_ENABLED=1`、NSIS には別の `TIKTOOLS_APP_UPDATE_NSIS_ENABLED=1` が必要です。既存コードや署名単体テストだけではリリースで有効化する根拠になりません。

## 期待される結果と提供状況 {#expected-result-and-status}

可搬型梱包と環境別インストーラーツールは実装済みです。確認・ダウンロードは設定に依存し、自動置換・再起動は実験段階です。不明・展開済み Linux 配置は手動インストールへ戻ります。NSIS を起動しても、終了する旧プロセスはインストール成功を報告できません。

## 検証と対処 {#verify-and-recover}

ローカルのプロトコル・署名検証には `bun scripts/task.ts rust test -p tiktools-app-update --locked` を使います。実際の更新、失敗、プロファイル保全、再起動には対応する使い捨て環境が必要です。前の検証済み成果物を保持し、フィード・補助ファイルの公開中断は安全に拒否します。プラグイン終了が不確実な間は引き渡しを再有効化しません。[リリース処理](https://github.com/nglmercer/TikTools-app/blob/remake/.github/workflows/release.yml) と更新ソースが条件の正本で、過去の計画は配置チェックリストではありません。

## 運用の詳細 {#operational-details}

ローカル portable は source/manifest/binary の版一致と widgets/web・release 実行体が必要です。例は現在版を使い、意図した版変更時だけ tag を変えます。`package` は binary/layout を検証し `release/TikTools-<version>-<platform>.tar.gz`（Linux）または `.zip`（Windows）を作ります。`RELEASE_BINARY` で実行体を指定できます。Windows は PowerShell の環境指定と `windows-x86_64` を使います。installer は release workflow の依存・固定 cargo-packager を準備し同じ tag/platform で `bun scripts/package-installers.ts` を実行します。portable だけでは NSIS/DEB/AppImage はできません。

コンパイル条件は、checks に `TIKTOOLS_APP_UPDATE_FEED_ENABLED=1` かつ有効 `TIKTOOLS_APP_UPDATE_PUBLIC_KEY`、AppImage はさらに `TIKTOOLS_APP_UPDATE_INSTALL_ENABLED=1` と対応元検出、NSIS はさらに `TIKTOOLS_APP_UPDATE_NSIS_ENABLED=1` が必要です。全てビルド時指定です。workflow は repository 変数 `TIKTOOLS_UPDATE_PUBLIC_KEY` を鍵・feed に対応させます。不在なら assets/checksums だけを公開でき、署名 feed は出しません。有る場合は保護 key/password・正確な manifest/package 署名・公開物検証・認証昇格が必要です。updater・OS installer・plugin 発行鍵の署名は別です。

既定試験は実 AppImage の受入を省きます。ignored 2 件は 4.0.0 を返す実 disposable fixture（現在の assertion）、FUSE、cargo-packager signer を要求し、置換には画面も必要です。`TIKTOOLS_TEST_APPIMAGE` を設定し一覧確認後、`preflight_runs_a_real_appimage` と `packager_replaces_a_running_disposable_appimage_without_touching_its_profile` を `-- --ignored --nocapture` で実行します。展開ツリーは mount の代わりになりません。置換・版・再起動・profile を別途確認します。NSIS は終了後の独立受入が必要で、旧プロセスでは成功証明できません。

```sh
bun scripts/task.ts widgets
bun scripts/task.ts frontend-build
bun scripts/task.ts rust build -p tiktools-desktop --release --locked
RELEASE_TAG=v4.2.2 RELEASE_PLATFORM=linux-x86_64 bun scripts/task.ts package
```

```powershell
$env:RELEASE_TAG = 'v4.2.2'
$env:RELEASE_PLATFORM = 'windows-x86_64'
bun scripts/task.ts package
```

```sh
TIKTOOLS_TEST_APPIMAGE=/absolute/path/disposable-v4.0.0.AppImage bun scripts/task.ts rust test -p tiktools-app-update --locked preflight_runs_a_real_appimage -- --ignored --nocapture
```
