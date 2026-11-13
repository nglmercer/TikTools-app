---
title: "ComboBox と SearchSelect"
docId: "developer.components.selects"
translationKey: "developer.components.selects"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "6dc5181ece21f73c6569b5d8e54c896c6df86390333a5240ef0696257e5e56d5"
translationRevision: "4995a9bb8d2aec3d9dec37cd40ad64a4f23eb264d773c2aa2882f4b997679d20"
---

# ComboBox と SearchSelect {#combobox-and-searchselect}

## 目的と前提条件 {#purpose}

ラベル付きの選択肢から安定した値を選びます。SearchSelect はラベルと値を検索し、未知の選択値はそのまま表示します。既存の Vue ワークスペースと i18n プロバイダーを使います。プレビューは分離したアプリフレーム内に実際の共有コンポーネントをマウントし、ドキュメントのテーマはデスクトップのグローバル CSS を読み込みません。

## サンプルを試す {#example}

1. プレビューを読み込むか、完全なデモを開きます。
2. 状態選択を切り替え、該当する場合に読み込み中、空、エラー、無効、長文、モバイル表示を確認します。描画に違いがない状態のために、コンポーネントに架空の挙動を追加することはありません。
3. 操作に応じて Tab、Enter、Escape、矢印キーを使います。アクセシブルな名前、見えるフォーカス、ポップアップを閉じたときのフォーカス復帰を確認します。
4. 値を変えるかアクションを実行し、選択や出力の結果を確認します。再読み込みか「デモをリセット」でデモの状態が戻ります。

<DemoFrame scenario="settings" locale="ja" route="/components/selects" title="ComboBox と SearchSelectのサンプル" />

## API {#api}

以下の props、既定値、イベント、スロット、モデル宣言はコンポーネントのソースから生成されます。インポートされた型は手作業で複製せず、元のソースに関連付けたままにします。未宣言の属性はコンポーネントの転送契約に従います。ラッパーにネイティブイベントを結び付ける前にソースを確認してください。

<!--@include: ../../../../.generated/components/selects.ja.md-->

## 実行されるソース {#source}

この同じファイルをプレビュー用にコンパイルし、コード例としても読み込みます。小さな状態はサンプルに属します。アカウント、プラグイン、ホストのロジックは共有 UI に入れず、features/entities に置きます。

<<< ../../../../examples/selects.vue

## アクセシビリティと対処 {#accessibility}

見えるラベルか aria-label を使い、キーボード操作を維持し、色だけに頼らないでください。狭い幅と長い翻訳ラベルでも確認します。フォームでは行の controlAttrs を実際の入力に結び付け、エラーと必須状態の関連を維持します。無効な操作欄だけでは失敗の理由は伝わりません。近くに状態と復旧操作を表示します。

プレビューが失敗したら閉じて開き直し、そのデモインスタンスで「デモをリセット」を使います。デモが使えなくても静的な API とソースを確認します。ネイティブダイアログ、外部サービス、OS 連携は別途デスクトップでテストする必要があります。[コンポーネントガイドに戻る](index.md)。

## 運用の詳細 {#operational-details}

読み込み中も選択項目を渡すとラベルを表示でき、不在なら生のモデル値を表示します。`search` は所有者の任意の遠隔検索へ query を通知し、内部はラベル・値の両方を照合します。矢印/Home/End は有効項目を移動し Enter が選択します。`allowCustom` の確定は前後空白を除き、Escape は編集を取り消します。未取得 ID・無効行・独自値も試してください。

## 運用の詳細 {#keyboard-contract}

[キーボード・dialog 例外・tooltip の命名](../i18n-accessibility.md#operational-details)を確認し、閉じられない dialog・unmount 草稿を含め全体の初期・復帰 focus を試してください。
