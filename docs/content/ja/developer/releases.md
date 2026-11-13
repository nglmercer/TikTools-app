---
title: "リリース・パッケージ・更新機能の有効条件"
docId: "developer.releases"
translationKey: "developer.releases"
locale: "ja"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "a606fb0f2723757e4fc3c25afb2c6b9f259bfdb5e988dd504b3f35d7d6a9f5fe"
translationRevision: "a606fb0f2723757e4fc3c25afb2c6b9f259bfdb5e988dd504b3f35d7d6a9f5fe"
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
