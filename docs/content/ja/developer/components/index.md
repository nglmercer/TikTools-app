---
title: "共有 UI コンポーネントと構成"
docId: "developer.components.index"
translationKey: "developer.components.index"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "7049a03b9a3b437dc969ee83208bbaac8521bd55bf0e557e96a185ffdfaff9f6"
translationRevision: "56ade304a45796024beb49a030c9a42a7e55fba3cbe70d08b501fd413bd8afb0"
order: 290
---

# 共有 UI コンポーネントと構成 {#shared-ui-components-and-composition}

## 目的と前提条件 {#goal-and-prerequisites}

既存 UI キットから機能を組み立て、操作と翻訳を維持します。インストール済み Vue ワークスペースのツールチェーンを使い、[フロントエンド構成](../frontend.md) を読んでください。このページはキットの案内です。正確な API の根拠は各コンポーネントのソース、props、テスト、専用リファレンスです。

## 適切な基本部品を見つける {#find-the-correct-primitive}

`src/web/shared/ui/` にはボタン、テキスト・日付・色の入力、テキストエリア、チェックボックス、スイッチ、コンボボックス、タブ、テーブル、フォーム、ダイアログ、状態、通知、構造用部品があります。デザイントークンは `src/web/app/styles/tokens.css` にあり、基礎・レスポンシブスタイルでアプリを構成します。機能ごとに別テーマを作らず、間隔、色、文字、フォーカス表現にトークンを使います。

1. 必要な操作を持つ既存機能を探します。
2. ラップする前に部品の `defineProps`、スロット、emit・model 値、テストを読みます。
3. ドメイン検証、保存状態、ホスト呼び出しは担当する feature/entity に置きます。
4. 対応する属性転送契約を通して、アクセシブルなラベルと操作属性を渡します。

script-setup コンポーネントでは実際のボタン API を使えます。

```vue
<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
const { t } = useI18n();
</script>

<template>
  <BaseButton variant="primary" type="submit">{{ t('common.apply') }}</BaseButton>
</template>
```

送信と検証は外側のフォームが担当します。`BaseButton` は `default`、`primary`、`danger`、`ghost` に対応します。破壊的な操作の色だけでは確認フローになりません。

## 期待される結果と提供状況 {#expected-result-and-status}

共有部品は一貫したキーボード、フォーカス、ARIA の実装を提供します。組み合わせの確認は必要です。プレースホルダーはラベルではなく、アイコンだけの操作には名前が必要です。無効状態は現在の操作の所有関係を反映し、非同期エラーは見えるままにします。開発用 UI キットルートは見本で、別製品の機能やバックエンドの受け入れテストではありません。

## 検証と対処 {#verify-and-recover}

`bun scripts/task.ts test web BaseButton`、変更部品の対象テスト、`bun scripts/task.ts frontend-lint` を実行します。狭幅、キーボード移動、翻訳ラベルを確認します。属性が入力ではなくラッパーに届く場合は、部品を迂回する前に `shared/lib/use-split-attrs.ts` と操作テストを確認します。専用サンプルを応用する際は[アクセシビリティ](../i18n-accessibility.md) と [UI ソース](https://github.com/nglmercer/TikTools-app/tree/remake/src/web/shared/ui) を参照します。

## 対話型リファレンス {#families}

- [ボタンとアクション](buttons.md)
- [メニュー](menus.md)
- [TextField と SpinBox](fields.md)
- [ComboBox と SearchSelect](selects.md)
- [スイッチ](switches.md)
- [フォーム](forms.md)
- [タブとツールバー](tabs.md)
- [リストとテーブル](collections.md)
- [フィルターと選択](filters.md)
- [通知・空の状態・ステータス](feedback.md)
- [パネルとダイアログ](panels.md)
- [FloatingPanel の使い方](floating.md)
- [Splitter と MasterDetail](layouts.md)
- [JSON 表示と差分](json.md)
- [インポートと確認](import-review.md)
- [Cloud の変更確認パターン](cloud-patterns.md)
- [オーバーレイのパターン](overlay-patterns.md)

## 運用の詳細 {#operational-details}

生成ファミリーは公開の合成用部品を意図的に選びます。[フォーム](forms.md)は SchemaForm/SettingsRow、[フィールド](fields.md)は textarea・slider・radio・tags・日付・色・単一ファイル、[メニュー](menus.md)は MenuBar/ContextMenu、[フィードバック](feedback.md)は Notification を含みます。アイコン・SchemaFieldRows 等は内部実装で独立公開契約ではありません。props・model と scoped slot bindings を読んでください。生成署名は検証・アクセシビリティ保証ではありません。
