---
title: "国際化とアクセシビリティ"
docId: "developer.i18n-accessibility"
translationKey: "developer.i18n-accessibility"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "2c74fcefd1d28353b1135c547b2e3e6ae852af8b683350011732d10c364f9323"
translationRevision: "e646dfb8972a1a6eb5bba44b50ba654f197906784f0a78b1003c9227c608851e"
order: 120
---

# 国際化とアクセシビリティ {#internationalization-and-accessibility}

## 目的と前提条件 {#goal-and-prerequisites}

対応言語で理解でき、キーボード、タッチ、支援技術で使える画面を追加します。[フロントエンドの層](frontend.md) を読み、既存の共有 UI を使います。実行時のアプリ言語と公開文書の言語版は別の仕組みです。

## 翻訳内容を追加する {#add-localized-content}

アプリのカタログは `src/web/shared/i18n/` にあります。英語が型付きの代替言語で、他の対応カタログはすべて遅延読み込みします。`activateLocale` が選んだメッセージを読み、`document.documentElement.lang` を更新します。別の local-storage キーを作らず、既存言語ストアで設定を保存します。

1. 意味に基づくメッセージキーを英語カタログと対応する各カタログ・モジュールに追加します。
2. API ID、イベントパス、保存リソース識別子は保ち、見えるラベルを翻訳します。
3. 可変テキストは補間を使い、表示値は共有の国際化ヘルパーを使います。
4. プラグイン記述子は既存の `{default,i18key}` 契約とホストが統合したカタログを使います。翻訳文字列やプラグイン提供文字列を HTML として描画しません。

## 操作の意味を保つ {#preserve-control-semantics}

`BaseTextField`、`BaseFormRow`、`BaseDialog`、タブ、状態部品を優先します。プレースホルダーはアクセシブルなラベルの代わりになりません。アイコンボタンには翻訳された名前が必要です。検証は対象フィールドと無効状態を示し、色だけで失敗を伝えません。ダイアログからのフォーカス復帰、Escape、キーボード選択、見える輪郭を維持します。

```sh
bun scripts/task.ts test web i18n
bun scripts/task.ts test web catalogs
bun scripts/task.ts test web BaseDialog
bun scripts/task.ts frontend-lint
```

手動確認では全手順を Tab でたどり、ダイアログを開閉し、開いたまま言語を変え、狭幅でも繰り返します。ホバーで現れる状態操作はキーボードフォーカスでも表示し、タッチでも使えるようにします。

## 期待される結果と提供状況 {#expected-result-and-status}

カタログの型と整合性検査は不足キーを検出し、共有操作部品はキーボードと ARIA の実装を提供します。組み合わせた全画面の包括的なアクセシビリティ認証ではありません。翻訳文字列が存在してもランキングとコンテストは非表示のままです。

## 問題が起きたとき {#recovery}

未翻訳が出たら、部品に英語の代替を追加する前に遅延読み込みとキーの綴りを確認します。フォーカスが消えたら転送属性の受信部品とダイアログの復帰先を確認します。現行動作は [i18n 実装](https://github.com/nglmercer/TikTools-app/blob/remake/src/web/shared/i18n/i18n.ts) と部品テストが定義し、[コンポーネント](components/index.md) が共有入口を説明します。

## 運用の詳細 {#operational-details}

言語優先順は保存 `tiktok-live-locale` → `navigator.languages` → 英語です。`normalizeLocale` は地域タグと `jp` → `ja` を扱います。コード/書式地域は `en/en-US`・`es/es-ES`・`pt/pt-BR`・`fr/fr-FR`・`zh/zh-CN`・`ja/ja-JP` で、`intl.ts` の formatCount/formatDate を使います。`activateLocale` はメッセージ/lang を変え、`useLocaleStore().setLocale` は保存と最善努力の設定 staging も行います。設定 UI は store を使います。英語 fallback、zh/ja は host メッセージも読みます。追加時は supported-locales・loader・catalog/host・地域対応・設定テスト・キー/placeholder/複数形/コンパイル試験・docs/site routing を更新し、API ID・引数名は保持します。

余白・字形・control・focus・層の token を使います。粗い pointer または幅 700px 未満で control は 44px、navigation は 900px 未満、form 行は 600px 未満で別々に変化します。reduced motion は時間を 0 にします。広い画面でも粗 pointer を試します。全体は dark で light override は JSON 限定です。

キーボード：tab は矢印/Home/End で自動選択、select は矢印/Home/End/Enter と Escape 取消、menu は trigger へ focus を返します。dialog は native showModal ですが `closable=false` は Escape を拒否し、閉じると内容を unmount します。草稿は所有者に置き初期・復帰 focus を確認します。icon button は `label` が必須で `hint`/tooltip は説明のみ。tooltip は hover/focus 500ms 後に現れ Escape で閉じ、dialog の top layer に入ります。実際の支援操作は手動確認し、unit test を画面全体の認証にしません。
