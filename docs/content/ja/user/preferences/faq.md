---
title: "よくある質問"
docId: "user.preferences.faq"
translationKey: "user.preferences.faq"
locale: "ja"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "efa3a64ab4d688fd7650801112a8d3200f2d685b223735f39c9163dbf095e6c4"
translationRevision: "c7994b9def793939c4c5afb4b1c93895ee25fc64f1f989d63460add3f098ee05"
order: 40
---

# よくある質問 {#frequently-asked-questions}

## 目的と前提条件 {#goal-and-prerequisites}

データや認証情報を変える前に、操作上のよくある混同を解消します。以下の確認には起動中のデスクトップアプリが必要です。対応パッケージは[動作要件](../requirements.md) を参照します。実験段階やサービス依存と明示されたものを除き、ここで扱うローカル機能は利用可能です。

## Cloud サインインで TikTok に接続できますか？ {#does-cloud-sign-in-connect-tiktok}

できません。[接続](../live/connections.md) で配信者を選び、別途 LIVE を開始します。Cloud ID はアカウント方式サービス用です。TikTok Cookie は接続用の専用マスク欄だけに入力します。

## 接続済みなのにフィードが空なのはなぜですか？ {#why-is-a-connected-feed-empty}

静かなルームでは新規イベントがないことがあります。全イベントを選び、検索を消してスクロールを再開し、接続状態を確認します。スクロール停止中も取り込みと自動化は動きます。フィード消去でポイントや保存統計はリセットされません。

## 統計は現在のセッションだけですか？ {#are-statistics-only-for-this-session}

現在の [分析](../points/statistics.md) は選択した配信者と期間の保存済み集計を照会します。日付を広げて更新すると過去の記録を調べられます。チャットの全記録ではありません。ポイント残高は別ルールで、手動や自動化の調整を含む場合があります。

## テンプレート・変数・ランキングはどこにありますか？ {#where-are-templates-variables-and-rankings}

テンプレートとプロファイルは [自動化](../automations/templates.md)、プラグインの「探す」はプラグイン用です。実行時の値は自動化の変数にあります。実験的なランキングとコンテストは `#/rankings` にありますが、メインナビゲーションと起動時復元から隠されています。通常のポイント順位表とフィードの順位パネルは別画面です。

## 保存するとすべて同期されますか？ {#does-saving-synchronize-everything}

されません。イベント／アクション変更はローカル基準との比較です。変数の保存はローカルで、対応時に別途確認付きの送信・採用・取り下げを行います。秘密値の比較はマスクされます。Cloud オーバーレイは作成・改訂とライブラリ操作を使い、この版に共有オーバーレイの変更アダプターはありません。方向を選ぶ前に [変更](../cloud/changes.md) を読んでください。

## テストは無害ですか？取り込むとすぐ実行されますか？ {#are-tests-harmless-and-do-imports-run-immediately}

テストは音声、ネットワーク要求、ポイント変更などを実行できます。まず [core.log](../automations/first-rule.md) を使います。取り込んだ定義は無効のままなので、確認後に明示的に有効化します。プラグインもインストールだけでなくサービス設定が必要です。

## 期待される結果と対処 {#expected-result-and-recovery}

リセットせずに担当ページを特定できるはずです。自動更新確認は任意で、自動インストールは実験段階かつ現行リリースの条件で無効です。完全なリモート共同操作と本番のニックネーム検索は配置と実動作確認に依存します。不明点が残れば[バックアップ](backups.md) を取り、バージョン、時刻、再現手順を含む[診断レポート](troubleshooting.md) を集めます。
