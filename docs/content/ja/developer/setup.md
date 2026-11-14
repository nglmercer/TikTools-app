---
title: "開発環境と開発サイクル"
docId: "developer.setup"
translationKey: "developer.setup"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "3dd24191fb33ac1ebfd42ca33e68a5a97537a89df09533c4588207ea6087daa5"
translationRevision: "62f30c303bb61e305480e537c6514a25828c7d7822b04798f5e8887e9826ef83"
order: 10
---

# 開発環境と開発サイクル {#development-setup-and-loops}

## 目的と前提条件 {#goal-and-prerequisites}

利用者の既存プロファイルを変えず、現在の `remake` ソースをビルドします。`package.json` の Bun 1.4.4、Rust 1.95 以降、ロック済み Vite に対応する Node `^20.19.0 || >=22.12.0` を使います。CI は現在 Rust 1.99.1 です。ソースのコンパイルと[バイナリ導入](../user/installation.md) は別です。

## 環境別の準備 {#platform-setup}

Windows は MSVC Build Tools、WebView2、同梱 OpenSSL 用のネイティブ Strawberry Perl を導入します。Cargo ラッパーは MSVC 環境とネイティブ Perl を検出し、明示的な `OPENSSL_SRC_PERL`/`PERL` 上書きを維持します。MSYS/Cygwin Perl はこの MSVC 経路に不適切です。LLVM の `lld-link` は任意です。

Ubuntu 24.04 の CI 依存関係には `libasound2-dev`、`libappindicator3-dev`、`libgtk-3-dev`、`libwebkit2gtk-4.1-dev`、`libxdo-dev`、`pkg-config` があります。AppImage 作成には GStreamer の開発情報、base/good/libav プラグイン、scanner も必要です。任意の `mold` は Linux リンクを高速化します。Android/Termux はこれらの代わりにネイティブ Android ツールチェーンを使います。

## 依存関係をインストールする {#install-dependencies}

```sh
git switch remake
bun ci
```

コミット済み Bun/Cargo ロックファイルを使います。`bun install` は意図的な依存変更用です。文書は明示的なワークスペースで、安定版 VitePress は独立した Vite を使い、アプリは Vite 8 を維持します。両方を同じメジャー版に強制しません。

## 開発サイクルを選ぶ {#choose-a-loop}

```sh
bun run start                 # Vite と Wry デスクトップ開発ホストを所有して起動
bun run dev                   # ネイティブのローカルブラウザー開発。開発プラグインの準備なし
bun scripts/task.ts serve     # Vite のみ。ネイティブ操作には実ホスト・アダプターが必要
bun scripts/task.ts packaged  # 開発 URL ではなく同梱フロントエンドのパスを使用
```

`start` は Vite を起動して割り当て URL を待ち、`TIKTOOLS_DEV_URL` でデスクトップを起動します。`TIKTOOLS_PLUGINS_REPO` または隣接する別の `plugins-example-tiktools` からプラグインを準備できます。このリポジトリ内のプラグイン実装はコンパイルしません。準備を省くには `TIKTOOLS_SKIP_DEV_PLUGINS=1` を使います。既存ホストは自動置換せず、デスクトップ所有者がいれば起動エラーになります。

起動前に外部プラグインリポジトリ自身の依存関係をインストールします。TikTools リポジトリで以下を実行すると、ロック済みのツールをインストールし、ホストを起動せずにパッケージを準備し、`dist/plugins/` に配布用アーカイブを作成できます。準備時は UI をソースから再ビルドし、`.dev-plugins` の生成内容を置き換えます。`TIKTOOLS_PLUGINS_REPO=off` はこのステージングディレクトリを空にし、外部の開発パッケージを読み込みません。ユーザーがインストールしたプラグインは別に管理されます。

以下の隣接ディレクトリのパスはローカル開発の例です。CI はこのプラグインリポジトリを必要としません。配置が異なる場合は `TIKTOOLS_PLUGINS_REPO` に絶対パスを指定します。

```sh
bun install --frozen-lockfile --cwd ../plugins-example-tiktools
bun scripts/task.ts prepare-plugins
bun scripts/task.ts plugins --all
```

分離実験では起動前に `TIKTOOLS_HOME` を使い捨てディレクトリへ設定し、操作する全 CLI でも同じホームを使います。稼働中ロックを削除しないでください。

## 検証と対処 {#validate-and-recover}

```sh
bun scripts/task.ts frontend-lint
bun scripts/task.ts frontend-test
bun scripts/task.ts frontend-build
bun scripts/task.ts rust fmt --all -- --check
bun scripts/task.ts rust-lint
bun scripts/task.ts rust test --workspace --locked -- --test-threads=1
```

編集中は対象テストを使い、マージ前には必須 CI 条件をすべて維持します。Cargo を呼ぶ署名テストの計測中、同じ target に複数の Cargo ビルドを走らせません。古いホストで起動が拒否されたら所有プロセスを正常停止します。リンカーや WebView 依存がなければアプリロジック変更前に環境を直します。
