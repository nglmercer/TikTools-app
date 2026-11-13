---
title: "プラグインの梱包・署名・レジストリ"
docId: "developer.plugins.packaging-registry"
translationKey: "developer.plugins.packaging-registry"
locale: "ja"
audience: "developer"
featureStatus: "available"
contentStatus: "reviewed"
sourceLocale: "en"
sourceRevision: "3d15fcb4c2e2528f60cab585a24e6d7172a4a4ca53905af1bc44cab5284edbcc"
translationRevision: "3d15fcb4c2e2528f60cab585a24e6d7172a4a4ca53905af1bc44cab5284edbcc"
order: 280
---

# プラグインの梱包・署名・レジストリ {#plugin-packaging-signing-and-registries}

## 目的と前提条件 {#goal-and-prerequisites}

バイト列、発行者権限、ホスト互換性を別々に確認できるパッケージを配布します。検証済みプラグインと対応対象から始めます。署名の秘密鍵はリポジトリ外に保管し、レジストリルートの委任は運用者が別途担当します。

## 確認可能なアーカイブを準備する {#prepare-a-reviewable-archive}

```sh
tiktools-plugin validate --publish
tiktools-plugin build --release
tiktools-plugin keygen --id acme-1 --publisher acme --out /private/path/keys
```

`build` はコンパイルして `.plugin` アーカイブを作ります。マニフェストの入口、アセット、ロケール、UI 出力、必要なランタイム依存を `checksums.json` とともに含めます。native/process は対象に合う成果物が必要です。JavaScript のみなら可搬性を持ちますが、ネイティブアドオンや環境機能を使うと制約が加わります。

既存のビルド済みアーカイブでは、未確認のワイルドカードではなく正確なパスを使います。

```sh
tiktools-plugin sign dist-plugin/acme.hello-0.1.0.plugin --key /private/path/keys/acme-1.key.json
tiktools-plugin verify dist-plugin/acme.hello-0.1.0.plugin --trust /private/path/keys/acme-1.pub.json
```

アーカイブ名を実際の対象別出力に置き換えます。署名はチェックサムを保護し、チェックサムは各ファイルを保護します。公開鍵は共有できますが、秘密鍵をソース管理やログに含めてはいけません。

## 信頼と更新方針 {#trust-and-update-policy}

静的レジストリはルート署名の権限と、委任された索引・発行者鍵を持ちます。発行者の名前空間承認とパッケージ完全性は別で、Cloud カタログのダイジェストはどちらの署名連鎖の代わりにもなりません。解決器はチャンネル、エンジン、スキーマ・プロトコル・ABI、ランタイム、対象を検査します。取り下げ、取り消し、非互換の版はファイルハッシュが一致しても対象外です。

インストール時は先に確認を表示します。自動更新には許可された方針、固定されていないパッケージ、互換の署名済み発行者、新権限がないことが必要です。権限拡大は確認が必要です。製品の Cloud カタログからのインストールと低水準の静的レジストリ作成は別で、通常の製品フローは任意のリモート URL を拒否します。

## 期待される結果・復旧・検証 {#expected-result-recovery-and-verification}

インストーラーは確認済みパッケージの適用前にバイト列を検証し、ロールバック・復旧状態を保持できます。開発用の配置は管理対象外です。信頼エラーでは未署名許可フラグを通常方針にせず、委任や発行者情報を修復します。起動が不確実なら置換を止め、保持した結果を照合します。

オフラインの検証には `bun scripts/task.ts rust test -p tiktools-plugin-package --locked` とレジストリテストを使います。技術形式の正本は[パッケージ検証器](https://github.com/nglmercer/TikTools-app/tree/remake/crates/tiktools-plugin-package/src) と[レジストリモデル](https://github.com/nglmercer/TikTools-app/blob/remake/crates/tiktools-plugin-registry/src/model.rs) です。公開基盤は運用者による別の受け入れ確認が必要です。
