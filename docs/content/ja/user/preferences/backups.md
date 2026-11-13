---
title: "ローカル作業のバックアップと復元"
docId: "user.preferences.backups"
translationKey: "user.preferences.backups"
locale: "ja"
audience: "user"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "b2ae789f9c15df21b415b5bc006404dffd60430f193fdaba8cd94931e750eedc"
translationRevision: "2d6dc0f1ddc110b2a313568a12ee7ee77d4572156151d36e6a12dce4e38acf82"
order: 20
---

# ローカル作業のバックアップと復元 {#back-up-and-restore-local-work}

## 目的と前提条件 {#goal-and-prerequisites}

更新、リセット、PC 移行の前に正常な構成を保存します。管理できる保存先と、TikTools を正常終了する時間が必要です。現在の UI に汎用の **すべてをバックアップ** ボタンはありません。個別の書き出しと、アプリを閉じたデータコピーを使います。

## 再利用可能な定義を書き出す {#export-reusable-definitions}

1. **自動化** で目的のプロファイルを選び、書き出しから `.tikprofile.json` を保存します。バージョンと日付を名前に含めます。
2. 単体テンプレートは、提供されている場合にテンプレートダイアログのダウンロード・書き出しから `.tiktemplate.json` を保存します。
3. 共有前に内容を確認します。テンプレートには送信先や設定値が含まれ得ます。プロファイル書き出しは残高、統計、実行時変数、メディア、アカウント認証情報のコピーではありません。

## ローカルデータをコピーする {#copy-the-local-data}

1. LIVE と共同操作の提供を止め、通常の方法で TikTools を終了します。ホストの書き込み停止を待ちます。
2. 使用中のデータルートを特定します。既定は Windows の `%LOCALAPPDATA%\TikTools`、Linux の `~/.local/share/TikTools`、または設定された `XDG_DATA_HOME` 配下の `TikTools` です。独自の `TIKTOOLS_HOME` やディレクトリ別上書きで場所は変わります。
3. ルートと、別途上書きしたデータ、プラグイン、プラグインデータ、プラグインストアのディレクトリを日付付き保存先にコピーします。`.db` 1つだけを選ばず、残っている SQLite 補助ファイルも含め、閉じた DB ディレクトリ全体をコピーします。
4. 参照する音声ファイルは別途保管します。音声アクションは音そのものではなくパスを保持します。アプリのバージョンと独自ディレクトリの場所も記録します。
5. ローカル設定には非公開値が含まれ得るためコピーを保護します。Cloud 認証は OS の認証情報ストアにあり、このフォルダーコピーでは移りません。

## 復元して検証する {#restore-and-verify}

TikTools を閉じたまま、現在のフォルダーを別の復旧コピーとして保管します。保存ファイルを目的のパスへ戻し、互換バージョンを起動します。再接続前にプロファイル選択、残高、変数、メディアパスを確認します。可搬型の定義は自動化で書き出したプロファイル・テンプレートを取り込みます。明示的に有効化するまで定義は無効です。

## 期待される結果・提供状況と対処 {#expected-result-status-and-recovery}

終了中のコピーと記載した入出力機能を復旧に使えますが、全バージョン間の復元を保証するものではありません。Cloud 変更は完全なバックアップではありません。復元先の版がデータを拒否したら停止し、保管したコピーに戻します。古いバイナリを再インストールしても DB 移行は戻りません。[更新](../updates.md) と[トラブルシューティング](troubleshooting.md) を参照してください。

## 運用の詳細 {#operational-details}

個別のパス指定は `TIKTOOLS_HOME` より優先されます。`TIKTOOLS_DATA_DIR` はデータベース、`TIKTOOLS_PLUGINS_DIR` はパッケージ、`TIKTOOLS_PLUGIN_DATA_DIR` はプラグイン状態、`TIKTOOLS_PLUGIN_STORE_DIR` はパッケージストアです。`TIKTOOLS_LOG_DIR` と `TIKTOOLS_TEMP_DIR` はログと一時ファイルを選びます。相対パスは起動時の作業ディレクトリ基準です。バックアップ前に `tiktools api call system.info --json` で実パスを確認し、継承した変数も記録します。ホスト停止後にデータ・パッケージ・状態と既存 WAL/SHM をコピーします。ログはデータベースの代わりになりません。

`system.info` は実 home/dataDir を返します。他のディレクトリは別端末ではなくホストの実起動環境で解決します。`system.health` は集計 ready/未処理と LIVE 入口カウンターを提供します。
