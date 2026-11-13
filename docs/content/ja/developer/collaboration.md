---
title: "共同操作の権限と認証"
docId: "developer.collaboration"
translationKey: "developer.collaboration"
locale: "ja"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "cb0c8fc55416a0ade232a0272051d244ed5558d4b1f3aec87d37a7500af59475"
translationRevision: "cb0c8fc55416a0ade232a0272051d244ed5558d4b1f3aec87d37a7500af59475"
order: 190
---

# 共同操作の権限と認証 {#collaboration-authority-and-authentication}

## 目的と前提条件 {#goal-and-prerequisites}

承認済み端末が既存ホストで範囲を絞った操作を調査・実行できるようにします。テスト用所有者プロファイル、リモート対応ビルド、設定済み通信を使います。アカウントログイン、Cloud チーム所属、ローカル所有権、直接委任セッションは異なる権限です。

## 信頼の連鎖を確立する {#establish-the-trust-chain}

ローカル所有者用の `collaboration.*` 管理が設定、招待、保留要求、主体、権限付与、セッション、監査を所有します。リスナー設定や認証情報作成の前にスキーマを検出します。

```sh
tiktools api schema collaboration.configure
tiktools api schema collaboration.invites.create
tiktools api call collaboration.status '{}' --format json
```

招待はペアリングを始めるもので、それ自体では業務上の権限を与えません。所有者の承認で対応するセッションと権限が作られます。直接 HTTPS では検証済み証明書か明示的に固定したネイティブ信頼が必要です。ネイティブ Bearer とブラウザー Cookie の境界は異なります。最初の `collaboration.hello` がプロトコル、ホストインスタンス、主体、セッション、権限リビジョン、機能、交渉済み上限を結び付けます。

## 操作ごとに認可する {#authorize-each-operation}

対応リモートメソッドは許可リスト、機能、リソース制限、現在のセッション検査を通過する必要があります。ワークフローの無効化・削除は永続的な compare-and-set メタデータを使います。ポイント調整は加算型で独自の付与制限があります。リモート自動化の読み取りは限定された要約です。無効な作成者所有の下書きを保存しても、所有者の実行権限は得られません。

所有者の確認・採用と検査付き承認は、現在の定義、依存関係、出所を結び付けます。プレビューマニフェストは調査情報で、実行トークンではありません。通信相手の ID を恒久権限と扱わず、実行境界で付与とリソースの結び付きを再検証します。

## 期待される結果と提供状況 {#expected-result-and-status}

直接ネイティブ共同操作には範囲付き読み取り、一部の永続的変更、制限付き復旧が実装されています。Cloud teams/relay は別のアダプターを持ちます。文書化された Cloud リレー読み取りを完全な管理や下書き実行と説明してはいけません。残る委任処理、ドメイン書き込み、複数 PC の受け入れは実験段階です。経路に応じて[リモートクライアント](remote-client.md) と [Cloud](cloud.md) を参照します。

## 復旧と検証 {#recover-and-verify}

取り消しや本人情報の変更では購読を停止し、非公開の投影を破棄します。交渉済みドメインとカーソルの有効性に従って `collaboration.subscribe`、`snapshot`、`resume` を使います。応答が不確実なら元のコマンド ID を解決します。新しい ID で削除を再試行したり、古い受領記録から権限を復元したりしません。[認可ソース](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-collaboration/src) とレジストリ方針が境界を定めます。TLS 成功だけでは認可テストになりません。
