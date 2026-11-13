---
title: "OBS 用のオーバーレイを準備する"
docId: "user.overlays.index"
translationKey: "user.overlays.index"
locale: "ja"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "3141f778f3bf0e5c9e01415122e7863e9342c266a57bee9811d7a6b75b2c842b"
translationRevision: "8c177b6a08ad3beed33350fc8049032fa340f500296b8ada36a1a3be547b853e"
order: 10
---

# OBS 用のオーバーレイを準備する {#prepare-overlays-for-obs}

## 目的と前提条件 {#goal-and-prerequisites}

読みやすいウィジェットを設計し、プレビューを確認してから、ブラウザーソースを OBS に接続します。起動中の TikTools とオーバーレイゲートウェイ機能が必要です。既定のループバックアドレスを使う場合、OBS は同じ PC で動かします。ライブ出力には、対応する受信イベントか設定済みランキングソースも必要です。

<DocCallout title="まずデザインをプレビューする" tone="tip">

配信を設定する前に、サンプルエディターで操作を試してください。架空のプレビューは見た目の確認に使えますが、実際の LIVE 配信や OBS の受信を証明するものではありません。

</DocCallout>

## 設計とプレビュー {#design-and-preview}

<DocSteps>

1. **ウィジェットを選びます。** **オーバーレイ** でゲートウェイ状態を確認し、フォロー、ギフト、チャット、シェア、サブスク、いいね、ランキングを選びます。
2. **読みやすく整えます。** デザインインスペクターでレイヤー、表示文字、色、間隔、アバターを編集します。実際の配信解像度で、十分なコントラストと読みやすい文字サイズを確認してください。
3. **プレビューを確認します。** **プレビューを再生** で配置を確認します。「元に戻す」・「やり直す」でローカル編集を調整し、**デザインを保存** を選びます。未保存で種別を変えると破棄確認が表示されます。

</DocSteps>

<DemoFrame scenario="overlays" locale="ja" panel="overlays" title="オーバーレイのデザインを操作する" />

<DocSteps>

1. **ソース URL をコピーします。** プレビュー・ソースパネルの **URLをコピー** で、使用可能な URL を取得します。画面上の URL は認証情報がマスクされているため、コピー操作を使い、取得した URL は非公開にしてください。
2. **OBS に追加します。** OBS のブラウザーソースに URL を貼り付けます。TikTools を起動したまま、配信で使う前に対応イベントを1つ確認します。

</DocSteps>

既定ゲートウェイは `127.0.0.1:17452` です。ループバック URL はブラウザーソースを実行する PC 自身を指すため、別の PC にコピーしても自動的に元のホストへ届きません。ランキングウィジェットにはランキング ID の選択も必要です。[ランキングの状況](../points/rankings.md) を参照してください。

## Cloud ライブラリを使う {#use-the-cloud-library}

ホストが Cloud オーバーレイに対応していれば、オーバーレイからライブラリを開きます。ローカルデザインや **自分のオーバーレイ** を閲覧し、エディターで Cloud オーバーレイを作成するか、既存 ID を開いて改訂します。Cloud エディターはテキスト、ライブフィールド、画像、キャンバス寸法を扱い、画像アップロードは別フローです。ローカル保存と Cloud の作成・改訂は別操作です。

## 期待される結果と提供状況 {#expected-result-and-status}

ローカルデザイン、ゲートウェイ操作、ホスト機能に応じた Cloud 編集・ライブラリを利用できます。この版には共有オーバーレイの **変更** アダプターはありません。使用する OS 上の OBS でデザインと実際の LIVE 出力を確認してください。

## 問題が起きたとき {#recovery}

出力がなければゲートウェイ、アセット、認証情報、URL、イベント種別を確認します。Cloud のリビジョン競合は、現行版を再読み込みしてから編集します。ゲートウェイのエラーは[診断](../preferences/troubleshooting.md) から報告してください。

<DocCallout title="ソース URL には認証情報が含まれます" tone="warning">

ソーストークンを画像や問い合わせで公開しないでください。生成 URL のウィジェット専用認証情報を使い、実際に使う OBS ソースで無害なイベントを送り、受信を確認します。

</DocCallout>

## 運用の詳細 {#operational-details}

LIVE 用 OBS ソースを追加する前に、別配布の Event Gateway プラグインを導入・有効化し、実行中であることを確認します。状態ごとの対処は、`missing` → 導入、`disabled` → 有効化・対応状況の確認、`stopped` → 起動、`credentialUnavailable` → ゲートウェイを一度再起動、`assetsMissing` → 同梱ウィジェットアセットの修復・再構築、`starting` → 少し待って再試行、`unreachable` → ループバックのポートとサービスを確認、です。準備完了の状態やローカルプレビューだけでは、実際の LIVE イベントや OBS での受信を確認できません。
