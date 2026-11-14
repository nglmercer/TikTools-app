---
title: "リストとテーブル"
docId: "developer.components.collections"
translationKey: "developer.components.collections"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "45424c522635bc6271707b05d18c8d206878efbb9f9657dab9a9e69f3876832e"
translationRevision: "04ccbd751ddae7d69e36c3f91aef8d3cba85960d32dae3b77f7c930c4224c577"
---

# リストとテーブル {#lists-and-tables}

## 目的と前提条件 {#purpose}

安定したキーを選び、キーボードかポインターでレコードを操作します。並べ替えはモデルを変更し、所有側が順序付きの行を渡します。既存の Vue ワークスペースと i18n プロバイダーを使います。プレビューは分離したアプリフレーム内に実際の共有コンポーネントをマウントし、ドキュメントのテーマはデスクトップのグローバル CSS を読み込みません。

## サンプルを試す {#example}

1. プレビューを読み込むか、完全なデモを開きます。
2. 状態選択を切り替え、該当する場合に読み込み中、空、エラー、無効、長文、モバイル表示を確認します。描画に違いがない状態のために、コンポーネントに架空の挙動を追加することはありません。
3. 操作に応じて Tab、Enter、Escape、矢印キーを使います。アクセシブルな名前、見えるフォーカス、ポップアップを閉じたときのフォーカス復帰を確認します。
4. 値を変えるかアクションを実行し、選択や出力の結果を確認します。再読み込みか「デモをリセット」でデモの状態が戻ります。

<DemoFrame scenario="settings" locale="ja" route="/components/collections" title="リストとテーブルのサンプル" />

## API {#api}

以下の props、既定値、イベント、スロット、モデル宣言はコンポーネントのソースから生成されます。インポートされた型は手作業で複製せず、元のソースに関連付けたままにします。未宣言の属性はコンポーネントの転送契約に従います。ラッパーにネイティブイベントを結び付ける前にソースを確認してください。

<!--@include: ../../../../.generated/components/collections.ja.md-->

## 実行されるソース {#source}

この同じファイルをプレビュー用にコンパイルし、コード例としても読み込みます。小さな状態はサンプルに属します。アカウント、プラグイン、ホストのロジックは共有 UI に入れず、features/entities に置きます。

<<< ../../../../examples/collections.vue

## アクセシビリティと対処 {#accessibility}

見えるラベルか aria-label を使い、キーボード操作を維持し、色だけに頼らないでください。狭い幅と長い翻訳ラベルでも確認します。フォームでは行の controlAttrs を実際の入力に結び付け、エラーと必須状態の関連を維持します。無効な操作欄だけでは失敗の理由は伝わりません。近くに状態と復旧操作を表示します。

プレビューが失敗したら閉じて開き直し、そのデモインスタンスで「デモをリセット」を使います。デモが使えなくても静的な API とソースを確認します。ネイティブダイアログ、外部サービス、OS 連携は別途デスクトップでテストする必要があります。[コンポーネントガイドに戻る](index.md)。

## 運用の詳細 {#operational-details}

実行例は親に状態を置きます。表は sort model と computed で 3 行を並べ替え、見出し操作でも選択 ID を保持します。タブの草稿は key 付き子の外に保存し、入力・切り替え・復帰しても残ります。タブ・ダイアログ自体は草稿を永続化しません。親状態・常時 mount・KeepAlive を意図して選びます。
