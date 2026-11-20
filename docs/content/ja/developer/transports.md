---
title: "現在の通信経路と環境の制限"
docId: "developer.transports"
translationKey: "developer.transports"
locale: "ja"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6a5fcd19211bdd279a0696cde31c26cc7444a128e50c3ca752489b25acb7cd9a"
translationRevision: "11d31f98506b45ee544671e60ed105323f573e90405e126ab6c242d95a2ce964"
order: 200
---

# 現在の通信経路と環境の制限 {#current-transports-and-platform-restrictions}

## 目的と前提条件 {#goal-and-prerequisites}

対象ビルドと環境に実在する通信経路を選びます。[Control API](control-api.md) と[共同操作](collaboration.md) を読みます。通信は直列化した操作を運び、別コアや独自の権限を作ってはいけません。

## 境界を選ぶ {#choose-the-boundary}

| 経路 | 現在の実装 | 主な制限 |
| --- | --- | --- |
| デスクトップ WebView | Wry IPC と共有コントロールクライアント | GUI とスレッドの寿命はデスクトップ側 |
| ローカル CLI | Unix ソケットまたは Windows 名前付きパイプ、明示的な stdio ホスト | アプリのホームと所有者が同一 |
| ローカル ブラウザー | ネイティブ ループバック アダプターと `browser-transport.ts` | ペアリング済みネイティブ ホスト。公開リモートサーバーではない |
| 認証済み HTTPS/WSS ゲートウェイ | TLS・hello・権限・機能ゲートを備えた SDK/ブラウザー通信 | リモートコマンドには明示的な許可が必要 |
| インストール可能なリモート制御プラグイン | `remote.controller.ticket` を使う WSS リレー上の `tiktools.remote-control` | プラグインが所有し、リモートコードはホストにコンパイルされない |

TypeScript の通信インターフェースは `src/web/shared/api/control-transport.ts` にあります。リモート制御はホストバイナリの一部ではなく、インストール可能なプラグインです。すべてのコマンドは `HostCallPolicy`（`remote.command.dispatch`、`remote.pair.list`、`remote.grant.list`、`remote.grant.revoke`、`remote.controller.ticket`）を通過します。このビルドにはネイティブ peer/RTC 通信も Cloud Teams のホスト連携もありません。

## 選択した経路を調べる {#inspect-the-selected-route}

ペアリングや制御操作の表示の前に、接続済みホストが対応するメソッドと機能を確認します:

```sh
tiktools --json system info
tiktools api discover
bun scripts/task.ts test web browser-transport
```

ペアリングと許可はプラグインが所有します。コントローラー チケットは短期・単一回限りで、`remote.grant.list` / `remote.grant.revoke` が権威ある情報源です。リレー接続は通信経路を示すだけで、RPC 権限を示すものではありません。

## 期待される結果と状況 {#expected-result-and-status}

ローカル IPC とデスクトップ通信は確立済みです。リモート制御はインストール可能なプラグインと認証済みリレーで動作し、第 2 のホストランタイムではありません。ログインコードは 5 分の有効期限で接続済みデバイスに中継され、アカウント未設定時は OAuth にフォールバックします。Android のローカル IPC は計測対応 `mobile` ツール（`info` / `exec` / `tty`）で動作しますが、デスクトップ用ネイティブ アドオンと WebView は利用できません。

## 復旧 {#recovery}

切断は受け付けた操作の不確実性として扱います。再接続時は非公開の投影を無効化し、ID を再交渉し、有界ストリームを再開またはスナップショット化します。証明書チェックを緩めたり、リモート失敗後に黙ってローカル所有者にフォールバックしたりしてはいけません。期限切れチケットと失効した許可は fail-close にします。

## 運用の詳細 {#operational-details}

許可は最小権限かつ期限付きにし、再接続のたびに再確認します。ID、チケットの有効期限、許可の範囲、リレーへの到達性、フレーム サイズを個別に診断します。過大フレームは部分 RPC にならず、フレーム検証で拒否されます。実機 2 台の受け入れ前に、変更・失効したセッションを fixture で検証します。
