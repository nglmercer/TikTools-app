---
title: "ブラウザーデモの構造と制限"
docId: "developer.demo"
translationKey: "developer.demo"
locale: "ja"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "2e7f0ab271cce7a8c6e78d9d73c38ae0da022cefe2db104b1001a8f45cf8754c"
translationRevision: "4ad22e9703598e049dc6841b92058b827f8a59a5fa73831c99072885032a09f9"
order: 140
---

# ブラウザーデモの構造と制限 {#browser-demo-architecture-and-limits}

## 目的と前提条件 {#goal-and-prerequisites}

ブラウザーで合成データを使い、実際のアプリ UI を調べます。Bun 1.4.4 とリポジトリのロック済みインストールを使います。明示的なデモ入口は Rust、アカウント、起動中ホストから独立しています。ランタイムとブラウザー受け入れ検査はこのリポジトリに含まれます。

## ビルドと調査 {#build-and-inspect}

```sh
bun scripts/task.ts docs check
bun scripts/task.ts demo dev
bun scripts/task.ts demo build
bun scripts/task.ts site build
bun scripts/task.ts site preview
```

未コミットのローカルプレビューには `TIKTOOLS_SITE_ALLOW_DIRTY=1 bun scripts/task.ts site build` を使います。マニフェストは dirty と記録され、公開時には拒否されます。最終組み立てでは両バンドルに同じクリーンな Git SHA と base が必要なので、コミット後に再ビルドします。開発・プレビューサーバーはローカルブラウザー作業が必要なときだけ起動します。`docs` が VitePress、`demo` がブラウザーアプリのバンドルを所有し、`site build` が1つの静的サイトに組み立てます。文書側の VitePress 依存はアプリの Vite 8 から分離します。コマンドが黙って依存を導入したり Cargo を呼んだりしてはいけません。

## アプリの境界を維持する {#preserve-the-application-boundary}

`src/web/app/mount-workspace.ts` は共有 app/Pinia/router ファクトリーを提供し、インポートだけでマウントや通信選択をしません。明示的なデモ起動が共有 `ControlTransport`・制御クライアント境界に `src/web/demo/runtime/transport.ts` を渡します。純粋な初期フィクスチャーは Playwright から独立した `demo/fixtures/base-host.ts` にあります。`shared/lib/app-storage.ts` と `demo/storage.ts` の分離ストレージにより、「デモをリセット」がネイティブ設定や他のデモを消すのを防ぎます。ページ、ウィジェット、機能は既存 entity ストアと `src/web/shared/api` を使います。模倣アプリを別に作ったり、通常のネイティブ起動に偽 API を注入したり、デモ要求を実ホストへ流したりしません。

`demo/scenarios.ts` は11のシナリオ ID と許可ルート・言語・ツアーを検証します。特定の操作に絞った例では、`embed=connection|feed|automation|points|overlays` も指定できます。各埋め込みパネルは対応するルートを必要とし、ツアーは開始できません。不正な組み合わせは検証で拒否されます。埋め込み画面ではワークスペースのナビゲーションと完全版デモのツールバーを省き、実際のページコンポーネント、ストア、模擬ランタイムをそのまま使います。`runtime/host.ts` はハンドラーを明示登録し、検出にはその一覧を出し、`rpc-response` に要求 ID を返します。非対応の実処理は即座にエラーにします。架空レコードを初期化し、「次のイベント」ごとに固定時計を1刻み進めます。「再生」と「次のイベント」は必要なら先に模擬 RPC で架空配信者へ接続し、直ちにイベントを配信します。「再生」は2秒ごとに継続し、「一時停止」は生成を止めます。ツールバーは再生状態と件数を表示します。300刻みで停止し、フィードと実行履歴をそれぞれ最大60件保持します。応答待機列と保存レコードには別の上限があります。

「デモをリセット」は現在の `tiktools.demo.<instance>.*` 接頭辞だけを消し、フレームを再読み込みします。モジュールのシングルトン、ジャーナル、i18n、診断も初期化します。各埋め込みフレームに別のインスタンス ID を与えます。ページ退出時は Vue を解除し、ストアを破棄し、クライアント・通信を外して2つのタイマーを消します。`window.ipc` 橋渡しや任意の HTTP、shell、RCON、訪問者スクリプト、ネイティブ更新インストールは実行しません。CSP はネットワーク接続を拒否します。ログテンプレートとフィルター模擬は純粋な共有関数を使います。これはフロントエンド契約の例で、Rust 実行の代替ではありません。

共有入口 `startTour(id, {router, capabilities, taskState})` と公開 `tourRegistry` は `src/web/features/guided-help/index.ts` にあります。7つのバージョン付きツアーは安定したページ対象と、各対応言語の個別カタログを使います。要求時だけ Driver.js を読み、対象をワークスペース内に制限し、動きを減らす設定を尊重してフォーカスを戻し、移動時に上限付き待機を取り消します。閉じても作業完了にはならず、アプリ操作もしません。チュートリアル選択は作業全体を名指しし、対応シナリオを準備して再生を止めてから開始します。Driver の閉じる操作はガイドを終了し、ツールバーが結果を表示します。「デモをリセット」はこのインスタンスのバージョン付き進捗を消します。

## 期待される結果と提供状況 {#expected-result-and-status}

デモはシナリオモデルの範囲で移動、ローカル編集、イベント表示、復旧状態を示します。実 TikTok セッション、OBS ブラウザーソース、Wry・ネイティブ動作、本番バックエンド、複数 PC は検証しません。ランキングとコンテストは非表示、自動化の変更はローカル専用です。確認付き変数は実際のネイティブ製品実装、オーバーレイの変更は未マージの後続作業、更新自動インストールの条件は実験段階です。

## コンポーネントと Markdown {#components-and-markdown}

`docs/examples/*.vue` は描画カタログとソース例の両方を提供します。小さなボタンはデスクトップのグローバルスタイルなしで Markdown 内に描画します。状態付きダイアログ、配置例、アプリページは `DemoFrame` を使います。base を考慮した検証済み URL を作り、要求時だけ iframe を読み込みます。操作例は、それを説明する手順の近くに配置してください。

```vue
<DemoFrame scenario="connection" locale="ja" panel="connection" title="LIVE 接続パネルを操作する" />
```

`panel` プロパティで、対象を絞る `embed` クエリーと対応ルートを選びます。操作を始める前は軽量なプレビューを表示し、クリックすると実際の UI を読み込みます。フレームごとに分離されたインスタンス ID が割り当てられ、リセットはそのインスタンスだけに適用されます。完全版デモへのリンクは埋め込みモードを外し、ワークスペース全体を開きます。例を読み込む前でも記事が役立つよう、説明する手順と期待される結果は Markdown に残してください。公開用 `VITE_TIKTOOLS_PUBLIC_UI_KIT=1` は、デスクトップ開発ツール全体を有効にせず部品カタログを公開します。

## 復旧と検証 {#recovery-and-verification}

意図的な欠落・失敗の模擬後はシナリオを「デモをリセット」します。外部サービスへ接触したら停止して通信境界を修復します。内容、デモビルド、サイト組み立ては独立に確認します。文書ビルドだけではネイティブの正しさは証明できません。[テスト](testing.md) は偽ホスト、ブラウザー、Rust、手動受け入れの証拠を区別します。
