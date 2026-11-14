---
title: "スイッチ"
docId: "developer.components.switches"
translationKey: "developer.components.switches"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "fa20b0faee8aad6fcf04c58fbc0932d3a99c78667e38bbf3f3be007cadf0fce9"
translationRevision: "892546bb07a6f9f942a7b677b934c36f875382d120fa11a25b94b511b95dcec5"
---

# スイッチ {#switches}

## 目的と前提条件 {#purpose}

アクセシブルなスイッチで真偽値の設定を変えます。下書きと永続化を分け、保存エラーを近くに表示します。既存の Vue ワークスペースと i18n プロバイダーを使います。プレビューは分離したアプリフレーム内に実際の共有コンポーネントをマウントし、ドキュメントのテーマはデスクトップのグローバル CSS を読み込みません。

## サンプルを試す {#example}

1. プレビューを読み込むか、完全なデモを開きます。
2. 状態選択を切り替え、該当する場合に読み込み中、空、エラー、無効、長文、モバイル表示を確認します。描画に違いがない状態のために、コンポーネントに架空の挙動を追加することはありません。
3. 操作に応じて Tab、Enter、Escape、矢印キーを使います。アクセシブルな名前、見えるフォーカス、ポップアップを閉じたときのフォーカス復帰を確認します。
4. 値を変えるかアクションを実行し、選択や出力の結果を確認します。再読み込みか「デモをリセット」でデモの状態が戻ります。

<DemoFrame scenario="settings" locale="ja" route="/components/switches" title="スイッチのサンプル" />

## API {#api}

以下の props、既定値、イベント、スロット、モデル宣言はコンポーネントのソースから生成されます。インポートされた型は手作業で複製せず、元のソースに関連付けたままにします。未宣言の属性はコンポーネントの転送契約に従います。ラッパーにネイティブイベントを結び付ける前にソースを確認してください。

<!--@include: ../../../../.generated/components/switches.ja.md-->

## 実行されるソース {#source}

この同じファイルをプレビュー用にコンパイルし、コード例としても読み込みます。小さな状態はサンプルに属します。アカウント、プラグイン、ホストのロジックは共有 UI に入れず、features/entities に置きます。

<<< ../../../../examples/switches.vue

## アクセシビリティと対処 {#accessibility}

見えるラベルか aria-label を使い、キーボード操作を維持し、色だけに頼らないでください。狭い幅と長い翻訳ラベルでも確認します。フォームでは行の controlAttrs を実際の入力に結び付け、エラーと必須状態の関連を維持します。無効な操作欄だけでは失敗の理由は伝わりません。近くに状態と復旧操作を表示します。

プレビューが失敗したら閉じて開き直し、そのデモインスタンスで「デモをリセット」を使います。デモが使えなくても静的な API とソースを確認します。ネイティブダイアログ、外部サービス、OS 連携は別途デスクトップでテストする必要があります。[コンポーネントガイドに戻る](index.md)。
