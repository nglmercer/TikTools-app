---
title: "TikTools ドキュメント"
docId: site.home
translationKey: site.home
locale: "ja"
audience: all
featureStatus: available
contentStatus: reviewed
sourceLocale: "en"
sourceRevision: "17fab985404a52cd320675dd086c482ed54db4b0ab71457428b3c07b3c068d70"
translationRevision: "22356bb2fa9015c27d03d8de8d5b469d87de1211faa2d2cad3ce9b92e91b8812"
layout: page
order: 0
---

<DocsHome>

<template #eyebrow>

TikTools / LIVE ツールキット

</template>

<template #intro>

# あなたの LIVE を、<br />思いどおりに。 {#tiktools-documentation}

TikTok LIVE に接続し、視聴者への対応を自動化し、OBS オーバーレイをデザイン。架空のデータで実際の画面を試せます。

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
