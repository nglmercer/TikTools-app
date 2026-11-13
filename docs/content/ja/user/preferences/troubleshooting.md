---
title: "失敗した操作を診断する"
docId: "user.preferences.troubleshooting"
translationKey: "user.preferences.troubleshooting"
locale: "ja"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "41fe9f8a6082fad98b44a33890e9e275adc7a79aaf6cdd977e7424c459aa3ab2"
translationRevision: "41fe9f8a6082fad98b44a33890e9e275adc7a79aaf6cdd977e7424c459aa3ab2"
order: 30
---

# 失敗した操作を診断する {#diagnose-a-failed-operation}

## 目的と前提条件 {#goal-and-prerequisites}

処理を繰り返さず、失敗した要素を特定して役立つ報告を集めます。アプリのバージョン、おおよその時刻、試した操作が必要です。リセットやデータ置換の前に[バックアップ](backups.md) を取ります。

## 近くの証拠を確認する {#inspect-the-nearest-evidence}

1. 失敗箇所のエラーを読みます。診断状態の操作か **診断** から関連項目を調べます。
2. 発生元、レベル、文字列で絞ります。LIVE、監視、プラグイン、アカウント、同期、更新、通信、カタログ、インストール、共同操作が有用な発生元です。
3. 対象を選び、時刻、コード、要求・エンティティ識別子、短い説明を保持します。詳細があれば展開します。
4. 絞った診断をコピーするかテキスト報告を書き出します。失敗したら表示される代替テキストを使います。共有前にトークン、Cookie、招待リンク、個人的内容を除きます。
5. ページ、選択した配信者・プロファイル・プラグイン、操作、期待結果、実結果の短い再現手順を添え、更新前後のどちらかも説明します。

## よくある失敗を切り分ける {#isolate-common-failures}

空のフィードでは検索とフィルターを消し、スクロールを再開して[接続](../live/connections.md) を確認します。スクロール停止は切断ではありません。表示の欠落指標だけでポイントや統計も同じイベントを失ったとは判断できません。

自動化が実行されなければ、アクティブなプロファイル、イベント・アクションの有効状態、トリガー、フィルターのパス、クールダウンを確認します。処理を変える前に履歴を読みます。**テストを実行** は音声、HTTP、ポイント、プラグインを実行できます。

プラグインはインストールジョブか接続エラー、Cloud はアカウント状態と対象領域を確認し、TikTok Cookie を置き換えないでください。オーバーレイはゲートウェイとソースを確認します。更新のインストールが使えない場合は[手動更新](../updates.md) に従います。

## 期待される結果と提供状況 {#expected-result-and-status}

診断の絞り込み、コピー、テキスト書き出しを利用できます。配布アプリには web アセットと環境のランタイムも必要です。Linux は対応 GTK/WebKitGTK と X11/XWayland セッション、Windows は WebView2 を確認します。インストール範囲は[動作要件](../requirements.md) を参照します。

## 問題が起きたとき {#recovery}

原因を修正してから再試行します。リビジョン競合では現在の保存状態を再読み込みして確認します。未確認のリモートコマンドは再送前にジャーナル・状態を確認します。成功した処理の後でタイムアウトする場合があります。起動失敗時はデータを削除せず、パッケージ・バージョンと起動ログを報告します。

<DemoFrame scenario="diagnostics" locale="ja" tour="diagnostics" title="デモ: 診断" />
