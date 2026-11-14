---
title: "Cloud の契約・同期・確認"
docId: "developer.cloud"
translationKey: "developer.cloud"
locale: "ja"
audience: "developer"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "06ae46545d1366d4f456c78a0e6583ea0338afbd496467e4d25b3abe1a5430fb"
translationRevision: "34e604510645d6b1c0445da8ac73b8d16d37e497e327ef1c7e96ae7dd1b6d8a7"
order: 180
---

# Cloud の契約・同期・確認 {#cloud-contracts-synchronization-and-review}

## 目的と前提条件 {#goal-and-prerequisites}

用意された Cloud インスタンスに対してアカウント設定とカタログ機能を開発します。ローカルだけの運用も有効です。認証を伴う作業では専用のテストアカウント・バックエンドとネイティブのセッション引き渡しを使います。この文書は本番 Bearer や2端末での受け入れを確認するものではありません。

確認された契約の所有元は [nglmercer/RemakeBackend](https://github.com/nglmercer/RemakeBackend) で、[v2 設定契約](https://github.com/nglmercer/RemakeBackend/blob/master/docs/CLOUD-API-V2.md) と追加の [v3 API](https://github.com/nglmercer/RemakeBackend/blob/master/docs/CLOUD-API-V3.md) があります。バックエンド所有の可搬型の型とクライアント通信は、ローカルの `crates/cloud-contracts` と `crates/cloud-client` に対応します。デスクトップの耐久性と認証はホストの責任です。

## 操作の利用条件を確認し準備する {#gate-and-stage-operations}

1. `rpc.discover` でネイティブメソッドを検出します。古いホストでは従来の表示が残る場合があります。
2. 制限付き機能のエンドポイントを使う前に、ホストに `/api/v2/capabilities` を確認させます。通知されたサーバー機能をブラウザーだけのフラグに固定しません。
3. 運用者が用意したオリジンを使います。リリースビルドは実行時の任意オリジン置換を受け付けません。
4. 移行と競合をプレビューし、固定した基準と不確実な書き込みを保持して、確認済み操作だけを送ります。

```sh
tiktools api discover
tiktools api schema sync.v2.migrate
tiktools api schema globals.typed.replace
```

これらはスキーマを調べるだけで、アカウント移行や変数保存は行いません。

## 現在の実装 {#current-implementation}

V2 にはネイティブの状態・ストア・コーディネーター、保護付き v1 移行、競合確認、再構築、インストール済みプラグインの照合経路があります。採用は明示的かつアカウント単位です。移行印のない旧環境は旧経路を維持します。2台のデスクトップでの完全な受け入れと一部の製品アダプターは未完了です。アカウント全体の書き込みエポック設定には、移行確認で説明される不可逆なサーバー上の影響があります。

V3 は機能検出の下にカタログ、チーム、連携、アセット、オーバーレイ、パッケージを追加します。カタログ情報は署名付きプラグイン検証の代わりにはなりません。テンプレート・プロファイル取り込みは所有者ジャーナルを使い、適用・プロファイル取り込み時に enabled を確認します。パッケージ適用は中断手順を記録し、自動再実行しません。Cloud リレーの読み取り権限と直接共同操作の変更権限を混同しないでください。

## 期待される結果と対処 {#expected-result-and-recovery}

設定変更は ID、リビジョン、確認の境界を保持します。自動化の変更はローカル専用です。確認付き変数は型付きネイティブメソッドを使い、生の値を同期しません。オーバーレイの変更は未マージです。競合では基準・ローカル・最新 Cloud を比較し、応答不明では保持した意図を解決します。再試行可能な HTTP エラーでも変更の繰り返しは許可されません。実行中ホストの権限は別途[共同操作](collaboration.md) で扱います。

## 運用の詳細 {#operational-details}

`TIKTOOLS_CLOUD_ORIGIN` の優先順は debug で runtime → コンパイル値 → 既定、release はコンパイル値 → 既定で runtime を無視します。空・`off`・`none` は適用層だけで無効化します。HTTPS origin に限定し資格情報・パス・query・fragment を認めず、HTTP は明示 loopback のみです。分離 debug は `TIKTOOLS_CLOUD_ORIGIN=http://127.0.0.1:8787 bun run start`、release ビルドは `TIKTOOLS_CLOUD_ORIGIN=https://cloud.example bun scripts/task.ts rust build -p tiktools-desktop --release --locked` を使います。別 origin の保存 ID は復元せず、再認証前に確認します。コンパイル既定はサービス稼働の証明ではありません。

[移行とモード別回復](../user/cloud/changes.md#operational-details)に従います。不可逆 epoch 方針前に全端末をそろえ、件数・元結果を保持し、barrier 不明時は再読します。最初の成功確認のために barrier を繰り返しません。競合回復は確認入力・版を凍結し、古いレビューを作り直してから再開します。再開自体は Cloud 書き込みを送りません。
