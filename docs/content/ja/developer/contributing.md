---
title: "範囲を絞って変更に貢献する"
docId: "developer.contributing"
translationKey: "developer.contributing"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "2e0522178ebee69ea5eb20fec3c0ec586f13950c51d658cb3a21843536b2f9f9"
translationRevision: "58c0105316e54689625a140866a31ebc5ad44b40040d0ab0e71fd8960c240d80"
order: 230
---

# 範囲を絞って変更に貢献する {#contributing-focused-changes}

## 目的と前提条件 {#goal-and-prerequisites}

他の貢献者が確認・検証できる変更を準備します。編集前に [CONTRIBUTING.md](https://github.com/nglmercer/TikTools-app/blob/remake/CONTRIBUTING.md)、適用される `AGENTS.md`、[セットアップ](setup.md) を読みます。Bun 1.4.4、必要に応じ Rust 1.95 以降、コミット済みロックファイル、目的の対象から作ったブランチを使います。

## 変更を定めて実装する {#define-and-implement-the-change}

1. `git status` を確認し、他の担当者の作業を保全します。生成アセット、認証情報、ローカル DB を差分に含めません。
2. 現在の担当箇所に従います。サービス動作は Rust コア、契約は Control API モジュール、SDK 対応は型付きクライアント、表示は[フロントエンド構成](frontend.md) です。
3. 契約変更は正規の定義を更新し、派生出力も一緒に再生成します。別の完全な RPC・イベントスキーマ表を手で維持しません。
4. プラグイン固有実装は外部パッケージに置きます。汎用ホストは特定プラグインを取り込まず、マニフェストを検出します。
5. 具体的なきっかけと変更後の動作を、未完了・実験的な経路も含めて説明します。

## 変更に応じて検証する {#validate-proportionately}

```sh
bun scripts/task.ts frontend-lint
bun scripts/task.ts frontend-test
bun scripts/task.ts frontend-build
bun scripts/task.ts contracts --check
```

ネイティブ実装の変更には対応する Rust の整形、lint、テストと必須 CI 条件も必要です。文書のみの変更では docs 検査とコマンド・ソースリンクの確認を行い、ネイティブ実動作の受け入れを主張しません。対応入口はプロジェクトのタスクランナーが管理し、固有の対象フラグは Cargo 引き渡しを使います。

## 期待される結果と提供状況 {#expected-result-and-status}

レビューには焦点を絞った変更、有意な検証証拠、正直な制限を含めます。方針上の PR 対象は `remake`、リリースタグは保守担当者が管理します。貢献手順は利用できますが、ブランチ保護とリモート検査設定は管理者の責任で、Markdown ページが保証するものではありません。

## 問題が起きたとき {#recover}

依存関係や環境ライブラリ不足で検証が失敗したら、検査を弱めず環境を報告・修復します。生成物がずれたら所有元から再生成し差分を確認します。Cookie、鍵、認証情報、未処理のイベント記録を公開しません。ソースと古い設計が異なる場合は差を説明し、公開ガイドは現行実装に従います。広範な整理を一貫して見せるために他の担当者の未完了ファイルを上書きしないでください。

## 運用の詳細 {#operational-details}

review stamp は別々です。`sourceRevision` は現在の英語本文、`translationRevision` はこの記事の翻訳本文（frontmatter と外側空白を除去）の hash です。原文と全翻訳を確認して `bun docs/scripts/record-review.ts developer/example.md` を実行します。以後の本文編集は対応 stamp を無効にします。これは変化検出で、人間の ID・意味同等性の証明ではありません。コマンド・RPC/環境 ID・payload key・制限・有効化/回復・否定を明示比較し、技術指示を保持して全文章を訳します。

分割移行は元 topic/heading ごとに具体的な移行先を記録し、節なら fragment を付けます。checker はファイル・fragment・歴史 bytes を検証しますが、話題網羅は編集 review が必要です。成功のために歴史原文・provenance を書き換えません。
