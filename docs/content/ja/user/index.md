---
title: "TikTools を使う"
docId: "user.overview"
translationKey: "user.overview"
locale: "ja"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "72ce37218d4f3341fccd306bd0b1b4039ec6e1fd3dcfeb16f9096f43576c861d"
translationRevision: "72ce37218d4f3341fccd306bd0b1b4039ec6e1fd3dcfeb16f9096f43576c861d"
order: 0
---

# TikTools を使う {#use-tiktools}

TikTools は Windows または Linux のデスクトップを TikTok LIVE に接続し、イベントの表示、視聴者ポイントの保持、PC 上でのルール実行を行います。デスクトップ画面と任意のヘッドレスホストは同じ Rust コアを共有します。Cloud アカウントは TikTok 接続とは別です。

## 最初の作業を選ぶ {#choose-your-first-task}

1. [動作要件](requirements.md) を確認し、[公開済みバイナリをインストール](installation.md) します。デスクトップ版の利用に Rust、Cargo、Bun は不要です。
2. [LIVE に接続](first-live.md) します。ゲスト参加できるルームなら TikTok Cookie なしで始めます。
3. **ライブフィード** でイベントを確認します。**ポイント** は視聴者の合計、**分析** は統計を表示します。**自動化** はトリガーを設定済みアクションに結び付けます。
4. **プラグイン** から任意の連携をインストールします。外部プラグインはアプリと別に配布されます。
5. **オーバーレイ** で OBS のブラウザーソースを準備します。ウィジェットのライブ配信にはゲートウェイ連携が必要です。

## 利用可能な機能と実験的な機能 {#available-and-experimental-features}

メインナビゲーションにはライブフィード、ポイント、分析、接続、自動化、プラグイン、オーバーレイ、設定があります。ランキングと競技にはネイティブエンジンと直接アクセス用ルートがありますが、管理ページは通常のナビゲーションにまだ表示されません。古い設計書の試作機能がすべて公開済みとは限りません。

Cloud のプラグイン変更と確認付き変数操作は、各セクションの明示的な操作を使います。自動化では現在、ローカルの基準コピーと比較します。そのボタンでプロファイルを Cloud 公開することはありません。共同操作の通信と一部のインストーラーへの引き渡しは実験段階です。無効な操作欄は、多くの場合ホスト機能不足か前提条件の未達を示します。

## 問題が起きたとき {#when-something-fails}

**診断** を開き、エラーの発生元と要求 ID、インストール済みバージョンを記録します。Cookie、招待トークン、非公開の変数値は含めないでください。各ガイドの対処から始め、必要に応じてサポートガイドを使います。対話型デモは架空のデータを使うため、TikTok アカウント、OBS、ネットワークの動作を確認するものではありません。

ソース開発には別の[開発者向けドキュメント](../developer/index.md) を使ってください。
