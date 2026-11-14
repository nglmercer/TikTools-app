---
title: "ダウンロードと更新"
docId: "user.start.updates"
translationKey: "user.start.updates"
locale: "ja"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6b92351b3ee8eb1b0745db65bbddb3eab21f4ae0a9945795c94147c9ca347e73"
translationRevision: "6b4174370eed6e8c8781e7693df6e6c63645c22265163901bdd304860e2aa279"
order: 40
---

# ダウンロードと更新 {#downloads-and-updates}

## 目的と前提条件 {#goal-and-prerequisites}

更新確認と自動インストールを混同せず、適切なチャンネルを選んで更新します。版を変更する前、特に DB 移行が記載されている場合は、活動中のセッションを閉じてローカルデータをバックアップします。

## チャンネルを選ぶ {#select-a-channel}

[リリース一覧](https://github.com/nglmercer/TikTools-app/releases) は安定版とプレリリースを区別します。`-rc` などの候補は検証用です。アプリの更新機能は stable と beta のフィードを区別し、提示前に署名付きメタデータを認証します。未署名 JSON や改変インストーラーは受け付けません。

## 確認してインストールする {#check-and-install}

1. **設定 → 一般** で更新状態とインストール済み版を確認します。
2. 利用できる確認操作を実行します。公開鍵・フィード機能が未設定のビルドは利用不可を表示します。
3. リリースノートを読み、環境に合うパッケージを選びます。
4. ダウンロード・手動引き渡しが提供される場合はその経路に従います。リリース処理は公開鍵変数が構成された場合だけ認証付き確認を有効にし、自動インストールには追加条件があります。
5. 手動更新では TikTools を閉じ、検証済みの完全なパッケージで置換・インストールして再起動します。データとアプリファイルは分けて保持します。

## 期待される結果 {#expected-result}

再起動したアプリは選んだ版を表示し、想定したローカルプロファイルを読めます。可搬型 ZIP/TAR は手動置換が必要です。AppImage インストーラーがあっても、不明な構成や展開済み配置では手動が必要な場合があります。

## 復旧と制限 {#recovery-and-limitations}

署名検証の失敗は更新失敗です。公式コピーを取り直し、リリースのチェックサムを確認します。検証を迂回しないでください。AppImage 自動置換と NSIS 引き渡しには別々のビルド条件があり、実験段階です。一般的な自動ロールバックはありません。古いバイナリに戻しても移行済み SQLite スキーマは戻りません。

インストール済み版、形式、診断の更新エラーを報告します。署名鍵、招待トークン、アカウントデータは添付しないでください。対応する配置は[インストールガイド](installation.md) を参照します。

## 運用の詳細 {#operational-details}

更新機能はビルド時に固定され、確認には feed と公開鍵が必要です。AppImage 自動置換は共通 install 条件と対応した実インストール検出も要求し、NSIS は同じ共通条件に NSIS 固有条件も要求します。インストール後の環境変数では有効化しません。portable・展開・不明形式は手動置換です。実際の導入後に版・再起動・profile を確認し、download・checksum・起動だけを完了とみなしません。
