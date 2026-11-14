---
title: "TikTools ドキュメント"
docId: site.home
translationKey: site.home
locale: "ja"
audience: all
featureStatus: available
contentStatus: reviewed
sourceLocale: "en"
sourceRevision: "b4a9666a0af61d210e7e5ef1cfbc66b878599406ed8b56a491c4ceb63eb1a99a"
translationRevision: "285a9a3cdaca53a2a283059e662c6d06ad069a3a4a2cf7c066f3ffe594cc5d86"
layout: page
order: 0
---

<DocsHome>

<template #eyebrow>

TikTools / LIVE ツールキット

</template>

<template #intro>

# あなたの LIVE を、<br />思いどおりに。 {#tiktools-documentation}

TikTok LIVE に接続し、視聴者のイベントからアクションを自動実行。OBS 用のオーバーレイもデザインできます。下の実際の操作画面を架空のデータで試し、自分だけの配信環境を作りましょう。

</template>

<template #actions="{ demoHref }">

<a :href="demoHref" target="_self">TikTools を試す</a>
[TikTools をダウンロード](https://github.com/nglmercer/TikTools-app/releases)

</template>

<template #start>

## 次の LIVE で使ってみる {#start-here}

配信に取り入れる準備ができたら、Windows または Linux に TikTools をインストール。クリエイターに接続して、視聴者から届く最初のイベントを確認しましょう。

1. [**動作要件を確認** <span>自分の環境に合う構成を探します。</span>](./user/requirements.md)
2. [**TikTools をインストール** <span>公開済みバイナリを取得して開きます。</span>](./user/installation.md)
3. [**LIVE に接続** <span>ライブフィードで最初のイベントを確認します。</span>](./user/first-live.md)

</template>

<template #directory>

## 自分らしい配信環境へ {#explore-the-guides}

配信環境を整える手順から、独自プラグインでの拡張まで。

</template>

<template #user>

### [ユーザーガイド](./user/index.md) {#user-guides}

LIVE に接続し、視聴者にポイントを付与して、配信中の反応を自動化。最初のクリックから結果の確認まで、操作ごとのガイドで進められます。

- [最初の自動化を作る](./user/automations/first-rule.md)
- [OBS とオーバーレイを設定する](./user/overlays/index.md)
- [視聴者ポイントを理解する](./user/points/rules.md)

</template>

<template #developer>

### [開発者ガイド](./developer/index.md) {#developer-guides}

Rust ホスト、Vue フロントエンド、型付き Control API を扱います。アプリの開発や拡張はこちらから。

- [開発環境を整える](./developer/setup.md)
- [構造と契約を調べる](./developer/architecture.md)
- [最初のプラグインを作る](./developer/plugins/quickstart.md)

</template>

<template #resources>

## 使い続けるためのリソース {#resources}

- [**UI コンポーネントリファレンス** <span>17系統の操作例、表示状態、ソースコード。</span>](./developer/components/index.md)
- [**トラブルシューティング** <span>診断を見つけ、よくある問題から復旧します。</span>](./user/preferences/troubleshooting.md)
- [**リリースノート** <span>更新前に変更を確認します。</span>](./releases/index.md)

</template>

<template #footer>

ブラウザーデモは、実際の TikTools の UI を架空のデータで動かします。試すためのインストールやアカウントは不要です。実際の配信に必要な準備は、各ガイドで確認できます。

[GitHub のソース](https://github.com/nglmercer/TikTools-app)

</template>

</DocsHome>
