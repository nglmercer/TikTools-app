import type { ExperienceCopy } from '../types.ts';

const copy: ExperienceCopy = {
  interactive: '操作できるサンプル', fictional: '架空のデータ', tryIt: '操作を試す', reset: 'サンプルをリセット', openFull: '完全版のデモを開く', close: 'サンプルを閉じる',
  preview: 'プレビュー', loading: '実際の操作画面を読み込んでいます…', task: '試してみましょう', success: '確認する結果',
  topicsTitle: '操作しながら学ぶ', topicsDescription: '実際の画面で小さな課題を試し、結果を確かめましょう。次に学ぶ操作を選んでください。', readGuide: 'ガイドを読む',
  topics: {
    connection: { title: 'LIVE に接続する', description: '配信者を指定し、視聴者のイベントが届く様子を確認します。', task: 'demo_creator に接続し、受信イベントを確認してください。', result: '接続が有効になり、架空のイベントがフィードに表示されます。' },
    automation: { title: 'イベントをアクションにつなげる', description: 'チャットのルールを作り、実行結果を確認します。', task: 'サンプルのアクションを開いてテストし、実行結果を確認してください。', result: '自動化の履歴に、完了した実行と展開後の結果が表示されます。' },
    overlays: { title: '配信画面をデザインする', description: 'ウィジェットをプレビューし、OBS のブラウザーソースを準備します。', task: 'ウィジェットを選び、見た目を調整してプレビューを再生してください。', result: 'デザインがプレビューに反映されます。実際の OBS 配信には、起動中のゲートウェイが必要です。' },
    points: { title: '視聴者にポイントを付与する', description: '獲得ルールを設定し、視聴者の残高を確認します。', task: '獲得ルールを確認し、イベント前後の視聴者の残高を比較してください。', result: 'サンプルの残高が更新されます。デモの付与量は固定のため、実際の率は LIVE で確認してください。' },
  },
  otherTasks: {
    feed: { task: 'サンプルイベントを進め、フィードの項目を選んで視聴者のプロフィールを開いてください。', result: 'TikTok に接続せずに、架空のイベントの詳細と記録された視聴者の活動が表示されます。' },
    analytics: { task: '現在の合計を確認し、サンプルイベントを1つ進めて活動の変化を比較してください。', result: '模擬イベントが合計に反映されます。実際の配信を測定した値ではありません。' },
    plugins: { task: 'サンプルプラグインの内容と権限を確認し、模擬インストールの操作を試してください。', result: 'このデモ内でプラグインの状態が変化します。パッケージ、外部サービス、音声デバイスには接続しません。' },
    cloud: { task: 'ローカルの greeting と Cloud のサンプル値を比較し、確認した変更を選んで適用してください。', result: '架空の差分がローカルで解決されます。Cloud アカウントやリモートデータは変更されません。' },
    collaboration: { task: '架空のデバイスからの要求を確認し、必要な範囲に限定したアクセスだけを承認してください。', result: '確認した権限とともにサンプルの接続主体が表示されます。実際の外部デバイスはこのデモに参加しません。' },
    settings: { task: 'ローカル設定を変更して表示への影響を確認し、デモをリセットしてください。', result: '設定はこのデモのインスタンスだけに適用されます。リセットすると初期状態に戻ります。' },
    diagnostics: { task: '失敗した項目を選んで詳細を確認し、模擬アクションを再試行してください。', result: '外部サービスに接続せずに、サンプルの失敗と再試行の結果を追跡できます。' },
  },
  previewLabels: { creator: 'クリエイター', event: '視聴者のイベント', action: '自動化', output: '配信', chat: 'チャットメッセージ', gift: 'ギフトを受信', connected: '接続済み', disconnected: '未接続', message: 'こんにちは、TikTools！' },
  connectionStates: { label: '接続のサンプル', normal: '接続に成功', offline: '配信者がオフライン', auth: 'セッションが必要', offlineHelp: '配信者は現在 LIVE を行っていません。配信が始まったことを確認してから再試行してください。', authHelp: 'このルームには、許可された TikTok セッションが必要です。デスクトップアプリのマスクされた入力欄で、新しいセッション Cookie を使ってください。このデモに実際の認証情報を貼り付けないでください。' },
  releases: { stable: '安定版', prerelease: 'プレリリース', notes: 'リリースノート', source: 'ソースコード', downloads: 'ダウンロード', noDownloads: 'このスナップショットには、対応するインストーラーがありません。', checksums: 'SHA-256 チェックサム', published: '公開済みリリース', snapshot: 'リリース情報の確認日', refresh: 'GitHub で最新情報を確認', all: 'すべて', highlights: '主な変更点', unavailable: 'リリース情報を表示できません。GitHub の公開済みリリースを確認してください。' },
};
export default copy;
