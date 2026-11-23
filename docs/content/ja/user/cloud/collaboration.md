---
title: "共同操作とチームアクセスを準備する"
docId: "user.cloud.collaboration"
translationKey: "user.cloud.collaboration"
locale: "ja"
audience: "user"
featureStatus: "experimental"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "05f448dda78b05027b84b2c03f08a15eb515aee05af79397a1596d7e40ec5d40"
translationRevision: "8dc4eeba1d61d910188f76b01efa8e67d4e1e04a4def1b03952cf43823387e48"
order: 30
---

# 共同操作とチームアクセスを準備する {#prepare-collaboration-and-team-access}

## 目的と前提条件 {#goal-and-prerequisites}

所有者の管理権を保ちながら、別のオペレーターに定義された役割を割り当てます。リモートアクセスは設定のタブではなく、インストール可能な **Remote Control** プラグインが提供します。[Cloud ログイン](account.md)、対応するバックエンド、およびこのホストへのプラグインのインストールが必要です。まずは読み取り専用アクセスと本番以外の環境から始めます。

## リモートアクセスを設定する {#configure-remote-access}

1. `tiktools.remote-control` をインストールして有効にし、**Remote control** ページを開きます。
2. チームを作成し、**この PC** をホストとして登録して識別しやすいデバイス名を付けます。リレーリンクを開始し、ステータスが `ready` になるまで待ちます。
3. 招待を作成し、一度だけ表示されるトークンを信頼できる経路で渡します。招待は承認済みのアクティブセッションではありません。
4. 保留中のリクエストと要求された役割を確認します。期待するオペレーターだけを承認してください。付与はホストリンクが確認するまで `pendingHost` のままです。
5. 付与とデバイスを定期的に確認します。不要になった付与は取り消します。

委譲された定義の編集、所有者としての定義の採用、委譲された実行の承認は別々の判断です。保存されたソース、参照されるアクション、権限を対象のレビューで読みます。定義のプレビューだけでは副作用は許可されません。

## コントローラーデバイスを準備する {#prepare-a-controller-device}

コントローラー側のデバイスで独自のアカウントにログインし、コントローラーとして登録して招待トークンを引き換えます。所有者に保留中のリクエストが表示されます。ニックネーム検索には別途デプロイされたバックエンドのエンドポイントが必要で、現在のサービスでは利用できない場合があります。検索の失敗をアカウント不在と判断したり、選択を任意の ID で置き換えたりしないでください。

## 期待する結果とステータス {#expected-result-and-status}

2 アカウントのフロー（チーム、ホストリンク、招待、承認、読み取りコマンド、取り消し）はローカルバックエンドで検証済みです。設定の Collaboration/Teams タブは削除され、ネイティブの collaboration サービスにはこのビルドに Web UI がありません。本番のリレー/NAT 動作、ニックネーム検索、2 台の PC での LIVE 運用はここでは確認されていません。リンクの準備だけでは、目的の操作に対する権限の適用を保証しません。

## リカバリー {#recovery}

古い付与の場合はアクセスを取り消し、ホストリンクの更新後に再確認します。未確認のコマンドは、再試行前に記録された結果を確認します。セッション終了時はリレーリンクを停止するかアクセスを取り消します。招待トークンや認証情報は公開の[診断レポート](../preferences/troubleshooting.md)に含めないでください。

## 運用の詳細 {#operational-details}

すべてのリモートコマンドはホストのケーパビリティゲート（`remote.command.dispatch`、`remote.pair.list`、`remote.grant.list`、`remote.grant.revoke`、`remote.controller.ticket`）を通過し、プラグインは Control API や完全な bearer トークンを受け取りません。アクティブなトランスポートやチームへの参加だけではネイティブアクセスは付与されません。Observer は読み取り指向、moderator は一部の操作を追加し、automationEditor は一部の編集を追加し、custom には明示的にレビューされたポリシーが必要です。いずれも無制限のスクリプトや副作用を許可しません。現在の境界については[役割と確認の契約](../../developer/collaboration.md#operational-details)と[トランスポート](../../developer/transports.md)を参照してください。
