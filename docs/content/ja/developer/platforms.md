---
title: "対応環境とネイティブ Termux ビルド"
docId: "developer.platforms"
translationKey: "developer.platforms"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "c7befe2be6f91bcf50ce4572aaa95d02723a315ba11ed1d94e6d499e802c2473"
translationRevision: "87f6e7b9743b445490b995bd4bd10e65488704676890481e3b226d75005b4776"
order: 20
---

# 対応環境とネイティブ Termux ビルド {#platforms-and-native-termux-builds}

## 目的と前提条件 {#goal-and-prerequisites}

対象 OS に合うビルドを選びます。デスクトップ開発には Bun 1.4.4、Rust 1.95 以降と[セットアップ](setup.md) の WebView 依存関係が必要です。Termux は Android 専用ホームで Rust CLI を直接ビルドします。共有ストレージは実行ファイルや Unix ソケットに不適切です。

## 適切なホストをビルドする {#build-the-appropriate-host}

Windows は MSVC Build Tools と WebView2 を使います。同梱 OpenSSL にはネイティブ Strawberry Perl を使い、MSYS Perl で代用しません。Linux は GTK3、WebKitGTK 4.1、音声開発ライブラリ、`pkg-config` が必要です。Winit/Wry 連携は `crates/tiktools-desktop/src/platform.rs` に置き、コアの業務サービスをウィンドウツールキットから独立させます。

ARM64 端末では最初に `rustc -vV` が `aarch64-linux-android` を示すことを確認します。Termux で次を実行します。

```sh
pkg install git rust clang make cmake pkg-config python protobuf ca-certificates
export CARGO_BUILD_JOBS=2
bash scripts/termux/check.sh
cargo build -p tiktools-cli --locked
./target/debug/tiktools --standalone system info
```

ここでの Cargo は意図的です。端末内ビルドには Bun や Node は不要です。Bun が使えるデスクトップでは `bun scripts/task.ts rust ...` を通し、リンカー・キャッシュ処理を保ちます。端末の2つのターミナルでは `./target/debug/tiktools host --ipc` を起動し、以後の CLI も同じ `TIKTOOLS_HOME` を使います。

## 期待される結果と提供状況 {#expected-result-and-status}

ローカル IPC、SQLite、可搬型 process/QuickJS プラグインに Android 経路が実装されています。process 成果物は Android/Bionic 向けにします。ネイティブライブラリと `.node` アドオンは Android では利用不可です。デスクトップ音声、ホットキー、通知、ファイル選択、WebView の端末用バックエンドはありません。永続リモートプロファイルは認証情報ストアが未選択なので明示的に失敗します。SDK は認証情報をメモリー内に保持できます。

## 検証と対処 {#verify-and-recover}

ルートから `bash scripts/termux/doctor.sh`、`bash scripts/termux/test.sh`、`bash scripts/termux/smoke.sh` を実行します。省略したフロントエンド検査を正直に報告します。`protoc` の C++ シンボル問題はライブラリ変更前に `env -u LD_LIBRARY_PATH protoc --version` と比較します。CA バンドル不足は修復し、迂回しません。クロスコンパイルだけでは端末の受け入れを確認できません。[Termux スモークスクリプト](https://github.com/nglmercer/TikTools-app/blob/remake/scripts/termux/smoke.sh) に対象検査と制限があります。

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
