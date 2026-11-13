---
title: "OBS 用のオーバーレイを準備する"
docId: "user.overlays.index"
translationKey: "user.overlays.index"
locale: "ja"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6062b2d3d3248eaa9ab1c8f73082c8d4a6bbe81d199f8a45b6c8c28dea617d46"
translationRevision: "07cd5b4021af15df2fa78b7ad559ba6a0e72327a0e60c281df31852ab30b5bea"
order: 10
---

# OBS 用のオーバーレイを準備する {#prepare-overlays-for-obs}

## 目的と前提条件 {#goal-and-prerequisites}

読みやすいウィジェットを設計し、ローカルのブラウザーソースを OBS に接続します。起動中の TikTools、オーバーレイゲートウェイ機能が必要です。既定のループバックアドレスを使う場合、OBS は同じ PC で動かします。ライブ出力には対応する受信イベントか設定済みランキングソースも必要です。

## 設計とプレビュー {#design-and-preview}

1. **オーバーレイ** でゲートウェイ状態を確認し、フォロー、ギフト、チャット、シェア、サブスク、いいね、ランキングを選びます。
2. デザインインスペクターでレイヤー、表示文字、色、間隔、アバターを編集します。実際の配信解像度で十分なコントラストと読みやすい大きさを確保します。
3. プレビューを再生して配置を確認します。デザインの判断には使えますが、LIVE 配信の確認にはなりません。
4. 「元に戻す」・「やり直す」でローカル編集を調整して **デザインを保存** を選びます。未保存で種別を変えると破棄確認が表示されます。
5. プレビュー・ソースパネルで URL をコピーします。表示 URL は認証情報がマスクされているため、使用可能な URL はコピー操作で取得します。非公開にしてください。
6. OBS のブラウザーソースに貼り付けます。TikTools を起動したまま、配信で使う前に対応イベントを1つ確認します。

既定ゲートウェイは `127.0.0.1:17452` です。ループバック URL はブラウザーソースを実行する PC 自身を指すため、別の PC にコピーしても自動的に元のホストへ届きません。ランキングウィジェットにはランキング ID の選択も必要です。[ランキングの状況](../points/rankings.md) を参照してください。

## Cloud ライブラリを使う {#use-the-cloud-library}

ホストが Cloud オーバーレイに対応していれば、オーバーレイからライブラリを開きます。ローカルデザインや **自分のオーバーレイ** を閲覧し、エディターで Cloud オーバーレイを作成するか、既存 ID を開いて改訂します。Cloud エディターはテキスト、ライブフィールド、画像、キャンバス寸法を扱い、画像アップロードは別フローです。ローカル保存と Cloud の作成・改訂は別操作です。

## 期待される結果と提供状況 {#expected-result-and-status}

ローカルデザイン、ゲートウェイ操作、ホスト機能に応じた Cloud 編集・ライブラリを利用できます。この版には共有オーバーレイの **変更** アダプターはありません。OBS、実際の LIVE、各 OS での動作は自身の環境で確認する必要があります。

## 問題が起きたとき {#recovery}

出力がなければゲートウェイ、アセット、認証情報、URL、イベント種別を確認します。Cloud のリビジョン競合は現行版を再読み込みしてから編集します。画像でソーストークンを公開せず、ゲートウェイのエラーを[診断](../preferences/troubleshooting.md) から報告してください。

<DemoFrame scenario="overlays" locale="ja" tour="overlay" title="デモ: オーバーレイ" />

## 運用の詳細 {#operational-details}

LIVE OBS source 前に別配布 Event Gateway を導入・有効化し実行を確認します。状態別回復は missing=導入、disabled=有効化/対応確認、stopped=起動、credentialUnavailable=一度再起動、assetsMissing=同梱 widget assets 修復/再構築、starting=少し待つ、unreachable=loopback port/service 確認です。生成 URL の widget 専用 credential を使います。ready・ローカル preview は実 LIVE や OBS 受信を証明せず、意図した source で無害なテストを行います。
