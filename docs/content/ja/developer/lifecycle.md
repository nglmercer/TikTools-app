---
title: "ライフサイクル・所有権・後始末"
docId: "developer.lifecycle"
translationKey: "developer.lifecycle"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "3e866271067cf2c4d374aaccd9a6cdf6babf981df4deeff5313947cbf39b5cc7"
translationRevision: "fb98cb75916ac0080b8a309b82542be16f38c9cad99e80f9bcc9d8a70294dcc4"
order: 50
---

# ライフサイクル・所有権・後始末 {#lifecycle-ownership-and-cleanup}

## 目的と前提条件 {#goal-and-prerequisites}

マウント、再接続、終了時に所有リソースを解放します。[構造](architecture.md) を把握し、ネイティブの実験には分離した `TIKTOOLS_HOME` を使います。フロントエンドの購読解除とホスト終了は、異なる所有権の問題を解決します。

## デスクトップの寿命を追う {#follow-the-desktop-lifetime}

`crates/tiktools-desktop/src/app.rs` が Winit のアプリケーションハンドラーを実装します。ウィンドウと WebView はイベントループスレッドで作ります。最初は非表示で、Vue の `frontend-ready` 後に表示します。トレイがあれば閉じる操作は非表示にし、なければ終了を開始します。Quit はイベントループを抜ける前にコアへサービス停止を求めます。

`crates/tiktools-core/src/shutdown.rs` は早期にプラグイン起動の受付を閉じ、バックグラウンド調整と監視を止め、LIVE を切断して所有作業を排出します。複数の終了呼び出しは同じ完了を待ちます。Future を破棄しても任意のネイティブコードを止めたり、外部処理を取り消したりはできません。

## フロントエンドのリソースを明示的に所有する {#own-frontend-resources-explicitly}

`use-app-runtime.ts` で使われている方式に従います。

```ts
import { onUnmounted } from 'vue';
import { useControlClient } from '@/shared/api/use-control-client.ts';

const control = useControlClient();
const stop = control.onTopic('points.changed', () => {
  // この表示を所有するストアの確定済み状態をここで更新します。
});
onUnmounted(stop);
```

コールバックの内容は表示を所有する entity に属します。feature は購読、タイマー、アニメーションフレーム、要求スコープを破棄し、アプリの起動側は共有通信の接続・解除を担当します。ウィジェットテンプレート変更時は、古い通知タイマーを残さず以前のコントローラーを破棄します。

## 期待される結果と検証 {#expected-result-and-verification}

ページを往復してもコールバックが増えません。非表示は不要な描画を止めますが、権威ある計上は止めません。再接続は欠落を報告しスナップショットを再読込し、業務処理を再実行しません。対象例として `bun scripts/task.ts test web cloud-lifecycle` と `bun scripts/task.ts test web request-scope` を実行します。

## 復旧と制限 {#recovery-and-limits}

QuickJS 起動がタイムアウトしても結果が不明な場合があります。管理側はワーカー ID を保持して置換を禁止し、停止成功を装いません。一部の同期プラグイン制御は呼び出し側をブロックすることがあります。特に更新引き渡し時は実装上の制限として扱います。稼働中ロックを消さず所有ホストを正常停止します。[終了処理ソース](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-core/src/shutdown.rs) と実行時テストは、過去のライフサイクル図より強い根拠です。

## 運用の詳細 {#operational-details}

終了はまず LIVE 受け付けを停止して入口を排出し、観測処理の中止・切断とプラグイン polling 停止を行います。Analytics の flush、ポイント書き込み解決（30 秒）、SQLite checkpoint（5 秒）、ネイティブ終了へ進みます。polling timeout、集計失敗、未解決ポイント、checkpoint 失敗は更新終了を危険と判定します。`AppCore::update_shutdown_safe()` は終了完了と安全フラグの両方を要求し、完了だけでは不十分です。更新器はこのネイティブ結果を使い、未知の処理を認証するユーザー設定はありません。置換前にログ・回復データを保持し、不確実性を解決します。
