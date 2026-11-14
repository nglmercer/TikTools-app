---
title: "フォーム"
docId: "developer.components.forms"
translationKey: "developer.components.forms"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "45f58248a1b11283e61965ea97ddca26e664ef977cbe2115a601d3a029abab05"
translationRevision: "1089c0b282dd7dba651aadb66e4c946ea4c63f20359aa3b9676de4f4a4ee89a6"
---

# フォーム {#forms}

## 目的と前提条件 {#purpose}

ラベル、ヒント、エラーを実際の操作欄に関連付けます。フォーム行は自動の関連付けと、スコープ付き属性の明示指定に対応します。既存の Vue ワークスペースと i18n プロバイダーを使います。プレビューは分離したアプリフレーム内に実際の共有コンポーネントをマウントし、ドキュメントのテーマはデスクトップのグローバル CSS を読み込みません。

## サンプルを試す {#example}

1. プレビューを読み込むか、完全なデモを開きます。
2. 状態選択を切り替え、該当する場合に読み込み中、空、エラー、無効、長文、モバイル表示を確認します。描画に違いがない状態のために、コンポーネントに架空の挙動を追加することはありません。
3. 操作に応じて Tab、Enter、Escape、矢印キーを使います。アクセシブルな名前、見えるフォーカス、ポップアップを閉じたときのフォーカス復帰を確認します。
4. 値を変えるかアクションを実行し、選択や出力の結果を確認します。再読み込みか「デモをリセット」でデモの状態が戻ります。

<DemoFrame scenario="settings" locale="ja" route="/components/forms" title="フォームのサンプル" />

## API {#api}

以下の props、既定値、イベント、スロット、モデル宣言はコンポーネントのソースから生成されます。インポートされた型は手作業で複製せず、元のソースに関連付けたままにします。未宣言の属性はコンポーネントの転送契約に従います。ラッパーにネイティブイベントを結び付ける前にソースを確認してください。

<!--@include: ../../../../.generated/components/forms.ja.md-->

## 実行されるソース {#source}

この同じファイルをプレビュー用にコンパイルし、コード例としても読み込みます。小さな状態はサンプルに属します。アカウント、プラグイン、ホストのロジックは共有 UI に入れず、features/entities に置きます。

<<< ../../../../examples/forms.vue

## アクセシビリティと対処 {#accessibility}

見えるラベルか aria-label を使い、キーボード操作を維持し、色だけに頼らないでください。狭い幅と長い翻訳ラベルでも確認します。フォームでは行の controlAttrs を実際の入力に結び付け、エラーと必須状態の関連を維持します。無効な操作欄だけでは失敗の理由は伝わりません。近くに状態と復旧操作を表示します。

プレビューが失敗したら閉じて開き直し、そのデモインスタンスで「デモをリセット」を使います。デモが使えなくても静的な API とソースを確認します。ネイティブダイアログ、外部サービス、OS 連携は別途デスクトップでテストする必要があります。[コンポーネントガイドに戻る](index.md)。

## 運用の詳細 {#operational-details}

`BaseSchemaForm` は JSON オブジェクトモデルを扱い、永続化はしません。schema・`uiHints.fields`・`fieldOptions`・`fieldErrors` と任意の `suggestionsFor`・`openMediaPicker` を渡します。`showIf:{key,equals,notEquals}` は条件表示、`advanced` は追加項目のグループです。動的候補の取得・選択保持は所有者が行い、エラーはフィールド名で渡します。実媒体には picker が必要です。例には条件 textarea/media とグループを含みます。値がモデルへ入っても保存時の domain・host 検証は必要です。

`BaseFormRow` は `group=true` 以外では input を 1 個推定します。default slot の `controlAttrs:{id,aria-describedby,aria-invalid,aria-required}` を実コントロールへ渡します。複数の場合は `group` と個別名を使い、同じ id を配布しません。共通説明・エラーはグループへ付きます。`useSplitAttrs` は class/style を外枠、他の属性・listener をコントロールへ渡します。File upload は 1 ファイルの設定サイズ・型を検証し、失敗時はモデルを空にしてエラーを出します。選択自体は遠隔送信ではありません。
