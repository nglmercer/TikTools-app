---
title: "現在の通信経路と環境の制限"
docId: "developer.transports"
translationKey: "developer.transports"
locale: "ja"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6696987a0db16b3b4d364c6136d0e552dbebd6fc68346e3ed031587a7b004468"
translationRevision: "8c906798c481cda745d80d2b11ccfe73a2ab9d171780c591052631b292df8292"
order: 200
---

# 現在の通信経路と環境の制限 {#current-transports-and-platform-restrictions}

## 目的と前提条件 {#goal-and-prerequisites}

対象ビルドと環境に実在する通信経路を選びます。[Control API](control-api.md) と[共同操作](collaboration.md) を読みます。通信は直列化した操作を運び、別コアや独自の権限を作ってはいけません。

## 境界を選ぶ {#choose-the-boundary}

| 経路 | 現在の実装 | 主な制限 |
| --- | --- | --- |
| デスクトップ WebView | Wry IPC と共有制御クライアント | GUI・スレッドの寿命はデスクトップ所有 |
| ローカル CLI | Unix ソケットか Windows 名前付きパイプ、明示的 stdio ホスト | 同じホームと所有者 |
| ローカルブラウザー | ネイティブループバックアダプターと `browser-transport.ts` | ペアリング済みホスト。モックや公開リモートサーバーではない |
| 直接リモート | 認証付き HTTPS/WSS ゲートウェイと SDK・ブラウザー通信 | TLS、hello、権限、機能条件 |
| ネイティブ peer | `tiktools-peer`、暗号化 iroh/QUIC ストリーム | peer ID だけでは RPC 権限を得ない |
| ネイティブ WebRTC | `tiktools-rtc`、署名付きシグナリングとデータチャネル | 交渉、権限、経路確認も必要 |

TypeScript インターフェースは `src/web/shared/api/control-transport.ts` にあります。ネイティブ peer 設定は公開第三者リレーや探索を暗黙に有効化しません。WebRTC は Rust ネイティブ (`webrtc` 0.21.1) で、デスクトップ UI のブラウザー `RTCPeerConnection` 依存ではありません。

## 選択経路を調べる {#inspect-the-selected-route}

ペアリングやコントローラー操作の表示前に、接続ホストのメソッドと機能を検出します。

```sh
tiktools --json system info
tiktools api discover
bun scripts/task.ts test web browser-transport
bun scripts/task.ts test web remote-browser-transport
```

`tiktools-rtc/src/route.rs` は選択された ICE 候補対から直接・リレーを分類します。準備完了だけでは直接経路を証明しません。Cloud のシグナリング・リレー基盤は業務認可とは別で、ネイティブ権限検査を維持します。

## 期待される結果と提供状況 {#expected-result-and-status}

ローカル IPC とデスクトップ通信は確立済みです。ネイティブ peer、WebRTC、コントローラー連携にはソースと対象テストがありますが、配置と複数 PC の受け入れは実験段階です。peer/relay が未実装とする旧記述は過去のものです。Android のローカル IPC は動作しますが、永続リモートプロファイル用ストアは未選択で、ネイティブアドオンと WebView は使えません。

## 問題が起きたとき {#recovery}

切断後、受け付け済み操作は結果不明になり得ます。再接続では非公開投影を無効化し、ID を再交渉して上限付きストリームを再開・再取得します。証明書検査を弱めたり、リモート失敗から黙ってローカル所有者へ切り替えたりしません。[RTC 実装](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-rtc/src) と実際の機能グラフが古い計画に優先します。

## 運用の詳細 {#operational-details}

Peer は ALPN `tiktools/collaboration/1`、現在 15 秒の接続 timeout、明示構成した最大 4 relay を使います。Peer/RTC フレームは 1 MiB、RTC パケットは最大 16 KiB です。過大フレームは検証で拒否し、部分 RPC にはしません。構成・ID、信号処理、timeout、サイズ、認可を分けて調べます。Cloud relay は ICE TURN サーバーではなく、空リストは公開 discovery を有効にしません。ホスト・端末の稼働と現在の確認済み grant を確認し、ICE の直通/relay 経路を記録します。ready・暗号化だけでは RPC 権限も全 NAT 通過も証明できません。実機 2 台の前にセッション変更・失効と過大フレームを fixtures で検証します。
