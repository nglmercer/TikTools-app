---
title: "TikTools を開発する"
docId: "developer.overview"
translationKey: "developer.overview"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "1ee65a70172f8154eeaf60fb1a3069192769582a4ef264426d0aefaf346e8003"
translationRevision: "d30e9e185b058a920cbfcb31e41a7865112d9dea11de7ff1dd58acef2f34cc69"
order: 0
---

# TikTools を開発する {#develop-tiktools}

TikTools は型付き制御操作の背後に1つのネイティブ AppCore を置きます。Vue がデスクトップ、ローカルブラウザー、コントローラーの画面を描画し、Winit/Wry がウィンドウを所有します。CLI と SDK も同じ操作を使います。[開発環境](setup.md) を整え、変更を検証できる最小の開発サイクルを選びます。

## 入口を選ぶ {#choose-an-entry-point}

| 作業 | 入口 | 確認できること |
| --- | --- | --- |
| Vue 部品とストア | Vite、Vitest、注入した ControlClient | UI の動作。ネイティブウィンドウは未検証。 |
| デスクトップ連携 | `bun run start` | 開発用 WebView と IPC の連携。 |
| ネイティブサービス | 対象を絞った Cargo テスト | コア契約、永続化、サービス動作。 |
| ヘッドレスのブラウザー接続 | `bun run dev` | ネイティブホスト、ローカルアダプター、ペアリング付き Vite。 |
| ドキュメント | `bun scripts/task.ts docs dev` | Markdown と分離した VitePress ビルド。 |
| 公開デモ | 独立したデモ入口と模擬通信 | 配信元サービスではなく、決定的な架空シナリオ。 |

## ソースの担当範囲 {#source-ownership}

- `crates/tiktools-core`: サービス、操作、SQLite。WebView に依存しません。
- `crates/tiktools-control-api`: レジストリ、JSON-RPC ルーティング、IPC、機能検出。
- `crates/tiktools-desktop`: Winit/Wry、UI スレッド橋渡し、ライフサイクル、同梱アセット。
- `src/web/app`: 起動、ルーター、シェル。`pages` は `widgets` と `features` を構成し、`entities` はドメインストア・契約、`shared` は再利用 UI と基盤を所有します。
- `src/automation/contracts/generated`: Rust から生成したバージョン付きイベント契約。デモやガイドで同じスキーマを再定義しません。
- `packages`: 既存の TypeScript プラグイン SDK/UI パッケージ。プラグイン実装は外部リポジトリにあります。

## 貢献時の境界 {#contribution-boundary}

ネイティブ、ローカルブラウザー、リモートコントローラー、デモの入口を明示します。実環境の入口でホストがなければエラーであり、黙ってモックを選びません。所有スコープでリスナー、購読、タイマーを破棄します。Rust/Bun ロックファイルと必須 CI 条件を維持します。

利用者のインストールは[ユーザーガイド](../user/installation.md) にあります。開発時の検査だけでは実際の TikTok 接続、OBS 描画、ネイティブ WebView 互換性、2台の PC での共同操作は確認できません。
