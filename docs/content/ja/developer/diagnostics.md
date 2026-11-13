---
title: "診断と安全な証拠収集"
docId: "developer.diagnostics"
translationKey: "developer.diagnostics"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "bd492eda2007394d29d152e20ec0dba70614f06e6d29b75a3b741ed6b23f11b4"
translationRevision: "4c8b2b8ff62f3ab4bde2217b350936d3ef395dddfa74803ddacdc3b9edf62e02"
order: 130
---

# 診断と安全な証拠収集 {#diagnostics-and-safe-evidence}

## 目的と前提条件 {#goal-and-prerequisites}

アカウントの秘密を残さず、診断と復旧を混同せずに、原因特定に十分な証拠を集めます。非公開テストホストを使い、選択された[通信経路](transports.md) を把握します。アプリはセッション用収集器を1つ設置し、状態部品は描画中に記録を追加せず、そこへ移動します。

## 失敗を調べる {#inspect-a-failure}

1. 失敗した接続、プラグイン、同期、更新の状態操作を開きます。デスクトップのログ操作はホバーと同様にキーボードフォーカスでも表示されます。
2. 発生元、重大度、文字列で絞り、選択項目の文脈と繰り返し数を確認します。
3. 説明に役立つ選択項目か絞り込んだ項目をコピーします。書き出しは共有前に確認します。
4. ネイティブの健全性とフロントエンドの観測を比べます。

```sh
target/debug/tiktools --json system info
target/debug/tiktools --json system health
target/debug/tiktools --json system doctor
```

切断中の LIVE 検査は省略される場合があります。コンパイル済み機能は実行時の利用可否を証明しません。デスクトップでは非モーダルのパネル、狭い画面では戻り先付きの `/debug` を使います。ネイティブのテキスト保存は `system.saveFile`、ブラウザーはローカルダウンロードです。

## 保持の仕組み {#understand-retention}

`src/web/shared/diagnostics/log.ts` は履歴を2,000件、直列化 UTF-8 で2 MiB に制限します。同じ発生元・エンティティの連続イベントは5秒以内なら繰り返し数にまとめます。認識できる認証情報や URL の秘密箇所は保持前に編集除去しますが、ラベルのない第三者の秘密は常に認識できるとは限りません。生の設定、Cookie、RPC パラメーター、HTTP 本文を収集器に追加しないでください。

## 期待される結果と提供状況 {#expected-result-and-status}

表示器は制限付きセッション履歴と文脈への移動を提供し、永続監査ログではありません。再読み込み・アンマウントでメモリーは消えます。履歴消去は接続エラーや原因を直しません。Cloud 要求 ID は安全な失敗情報の照合に役立ちますが、収集範囲は設置した収集器に従い、すべての登録・ネイティブエラーを必ず捕捉するわけではありません。

## 検証と対処 {#verify-and-recover}

`bun scripts/task.ts test web diagnostics` を実行し、[収集器ソース](https://github.com/nglmercer/TikTools-app/tree/remake/src/web/shared/diagnostics) を確認します。合成値で秘密除去、古い項目の排出、反復イベントを再現します。競合、結果不明コマンド、認証期限切れは担当の復旧フローに戻ります。診断コピーは証拠収集であり、操作の再試行や新しい信頼方針の承認ではありません。
