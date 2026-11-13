---
title: "TikTools ドキュメント"
docId: site.home
translationKey: site.home
locale: "ja"
audience: all
featureStatus: available
contentStatus: reviewed
sourceLocale: "en"
sourceRevision: "cc1465f0b5da5cbbd2ff9782ed1163f9a7f793b3fedc837340d3896e3da9c990"
translationRevision: "1653dabe6ae22b50e22a47b0c0551df4bfd484cef897708392a9e83667aa4626"
layout: page
order: 0
---

<DocsHome>

<template #eyebrow>

TikTools / ドキュメント

</template>

<template #intro>

# より良い LIVE への<br />ガイド。 {#tiktools-documentation}

実際の画面を操作しながら、TikTok LIVE への接続、チャットの自動化、OBS の設定を学べます。架空のデータで試し、結果を確かめてから、自分の配信に取り入れましょう。

</template>

<template #actions="{ demoHref }">

<a :href="demoHref" target="_self">対話型デモを試す</a>
[TikTools をダウンロード](https://github.com/nglmercer/TikTools-app/releases)

</template>

<template #start>

## 最初の LIVE を一歩ずつ {#start-here}

公開済みアプリから始めましょう。開発ツールは不要です。

1. [**動作要件を確認** <span>自分の環境に合う構成を探します。</span>](./user/requirements.md)
2. [**TikTools をインストール** <span>公開済みバイナリを取得して開きます。</span>](./user/installation.md)
3. [**LIVE に接続** <span>ライブフィードで最初のイベントを確認します。</span>](./user/first-live.md)

</template>

<template #directory>

## 自分に合う入口を見つける {#explore-the-guides}

2つの入口から、同じツールキットへ。

</template>

<template #user>

### [ユーザーガイド](./user/index.md) {#user-guides}

配信を整え、視聴者に反応し、日々の作業を楽にします。アプリを使う方はこちらから。

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

## 作業中に役立つ資料 {#resources}

- [**UI コンポーネントリファレンス** <span>17系統のサンプルとソース。</span>](./developer/components/index.md)
- [**トラブルシューティング** <span>診断を見つけ、よくある問題から復旧します。</span>](./user/preferences/troubleshooting.md)
- [**リリースノート** <span>更新前に変更を確認します。</span>](./releases/index.md)

</template>

<template #footer>

デモは架空のデータを使います。各ガイドには実験的な機能と前提条件を記載しています。

[GitHub のソース](https://github.com/nglmercer/TikTools-app)

</template>

</DocsHome>
