---
title: "プラグインの権限と分離 UI"
docId: "developer.plugins.permissions-ui"
translationKey: "developer.plugins.permissions-ui"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "740e66bbac2e47784638dd501df8a42edf9942b0ee5d1e7475fb2d01027b75cc"
translationRevision: "740e66bbac2e47784638dd501df8a42edf9942b0ee5d1e7475fb2d01027b75cc"
order: 270
---

# プラグインの権限と分離 UI {#plugin-permissions-and-isolated-ui}

## 目的と前提条件 {#goal-and-prerequisites}

プラグインスクリプトをメインアプリへ取り込まず、設定と意図したホスト処理を公開します。検証済みマニフェスト、対応 SDK/UI パッケージ、非公開テストホストを使います。[ランタイムの信頼](runtimes.md) を読んでください。マニフェストの機能はネイティブライブラリや process 実行ファイル用の OS サンドボックスではありません。

## 提供範囲を絞って宣言する {#declare-a-narrow-contribution}

アクションのフィールドと設定は JSON スキーマと UI ヒントで記述します。単純な宣言ページは信頼済みのホスト部品を描画します。イベント要求には `events.publish`、ホスト音声には追加で `audio.play` 機能と `audio.output` 権限が必要です。網羅的な権限一覧を文書に複製せず、正規の[マニフェストスキーマ](https://github.com/nglmercer/TikTools-app/blob/remake/packages/plugin-sdk/plugin.schema.json) と Rust パーサーを確認します。

プロセッサーは宣言済みの情報付加機能を必要とし、副作用のない注釈・表示・ログを返します。ドメインオブザーバーは `events.subscribe` が必要です。上限付き配信で安定した包みを受け取り、Rust 内部列挙型や任意の通信ハンドルは受け取りません。

## 分離した独自ページを作る {#build-an-isolated-custom-page}

`apiVersion:1`、`mode:"webview"` のマニフェスト `ui` はコンパイル済み `ui/dist/` を参照します。メインアプリは不透明オリジンのスクリプト有効フレームを保持し、任意の別ウィンドウは制限付きネイティブブローカーを使います。独自スクリプトはメイン Vue 内で実行しません。

```ts
import { createBrokerClient } from '@tiktools/plugin-ui';

const broker = createBrokerClient();
const settings = await broker.getSettings();
const stop = broker.subscribe('plugin.settings-changed', () => location.reload());
window.addEventListener('pagehide', stop, { once: true });
```

ホストがプラグイン ID を渡し、クライアントは他の ID を選べません。ブローカーは自身の設定、アクション、候補、イベント、言語・テーマに限定されます。ブローカーなしで開いたページは明示的に失敗します。ページからのアクションは、ボタンが test と表示されても実処理です。

## 期待される結果と提供状況 {#expected-result-and-status}

宣言描画、埋め込みフレーム、ネイティブ別ウィンドウには契約とテストがあります。CORS/CSP、送信元ウィンドウの結び付け、ネイティブの許可リストが共同で境界を守り、Shadow DOM では守れません。WebKitGTK の変更不能 IPC プロパティは、ハンドラー自体が制限されているので削除不要です。

## 復旧と検証 {#recover-and-verify}

空白フレームでは方針を緩める前にアセットパス、CSP、不透明オリジンの CORS を確認します。`broker unavailable` ならホストされている文脈を確認します。`bun scripts/task.ts test bun plugin-ui` と `bun scripts/task.ts test web plugin-webview-host` を実行します。[ブローカーパッケージ](https://github.com/nglmercer/TikTools-app/blob/remake/packages/plugin-ui/src/broker.ts) が包み・バージョンと終了時処理の正本です。
