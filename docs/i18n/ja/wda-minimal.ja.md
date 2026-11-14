---
title: "WDA Minimal"
status: stable
updated: 2026-11-12
type: Reference
description: "WDA に組み込まれた最小の Design System。定義、token の由来、スターター構成、design.md の基準の出所を説明する。"
tags: []
source: docs/design-systems/wda-minimal.md
lang: ja
source_sha256: ea3928d64958d74f40926d431e835bd14c44e322991f778f5c8af37d3701ea5c
translated_at: 2026-11-12
---

# WDA Minimal

## 定義

WDA Minimal は WDA に組み込まれた最小の Design System であり、実プロジェクトの基準を検証する際の参照でもある。プロジェクトが所有するセマンティック Design Token、ネイティブなセマンティック HTML、Modern CSS を使い、vendor コンポーネントライブラリには依存しない。Spectrum 2 と共有する Lit を Custom Element の標準的な記述方法として採用する。

WDA Minimal は小規模でセマンティック、Web ネイティブで、プロジェクト token を中心に据える。実プロジェクトで利用でき、WDA Minimal Starter の作成にも使える。WDA Core、プロジェクト構成、token 処理、ビルド成果物、文書は、外部 Design System の vendor コンポーネントライブラリや専用ランタイムに依存しない。

初期化時の WDA Minimal プロジェクトには、ウェブサイトのパッケージ依存関係がない。その `package.json` は空の `dependencies` オブジェクトを持ち、`packageManager` フィールドもない。外部インストールを初めて行うまで、パッケージマネージャーのロックファイルは存在しない。WDA Minimal は vendor コンポーネントライブラリを含まず、スタータースクリプト（`scripts/main.ts`）も生成しない。

WDA Minimal は大規模な商用 Design System を置き換えるものではない。また、別のシステムを模倣するための抽象化レイヤーも追加しない。

## WDA における位置付け

オンラインで設定済みの Design System が見つからない場合、または Spectrum 2 の依存関係インストールに失敗した場合、WDA は固定された組み込みの WDA Minimal 一式にフォールバックする。同じリリースでは同じ一式を使う。WDA Minimal は参照用 Design System でもある。既定の latest stable Spectrum と並行して、異なる依存関係モデルとコンポーネントモデルを検証する。Spectrum 2 は vendor コンポーネントパッケージ六つと独立した Lit を含む。初期化時の WDA Minimal にはウェブサイトのパッケージ依存関係がなく、vendor コンポーネントライブラリも含まない。

WDA Minimal はアーキテクチャ上およびオフライン時の基準であり、Designer が初回起動時に見る視覚的な既定値ではない。既定の Design System は引き続き latest stable Spectrum である。オンラインで解決に成功すると、プロジェクトは正確なバージョンを記録する。既定のパッケージの解決またはインストールに失敗した場合、WDA は今回の実行ですでに書き込んだ Spectrum 専用ファイルと依存関係設定を削除してから、WDA Minimal プロジェクトを作成する。

`wda init` の実行時にオフラインである場合、または利用可能な対応パッケージマネージャーがない場合、初期化は依存関係のない完全な WDA Minimal プロジェクトを作成する。ローカルのパッケージキャッシュがあっても、初期化が Spectrum 2 に切り替わることはない。`anchor.json` は `initialization.fallback` の下に、本来の目標であった Design System（Spectrum 2）と、フォールバックの理由を記録する。stdout には実際の結果とパッケージマネージャーが示した理由が表示され、後から既定のパッケージを追加するための WDA Skill が案内される。Skill はユーザーの同意を得た後にだけ追加し、`wda init` を再実行しない。これによりプロジェクトをオフラインでも利用でき、Standard Web のデプロイ成果物も損なわれない。

WDA は WDA Minimal と Spectrum を比較し、依存関係モデルとコンポーネントモデルを検証するとともに、Core が特定の Design System に結び付いていないことを確認する。この比較は自動移行や損失のない移行を約束するものではない。

## Token の由来

token の値は `@adobe/spectrum-tokens` 15.2.0 の固定スナップショットに由来し、上流の更新は追跡しない。このスナップショットは npm registry の `dist-tags.latest` に基づき、バージョンは 15.2.0 である。

WDA がコピーするのは token の値であり、名前ではない。名前は引き続きプロジェクトがセマンティック名として定義する。例として `color.brand.primary` と `spacing.md` がある。

フォントファミリー token は Noto で始まる font stack を使う。Noto は SIL OFL 1.1 でライセンスされている。

リポジトリルートの `NOTICE` ファイルに著作権表示を記録する。Apache-2.0 の第 4(d) 項に従い、上流パッケージ名、正確なバージョン、ライセンス、変更内容を記載する。

スナップショットは自動更新されない。更新するには新しいバージョンを確認し、`NOTICE` を更新する。

## Tokens

`tokens/tokens.json` は、プロジェクトの標準 token の DTCG ソースである。`wda init` は Minimal Starter に必要なセマンティック token を作成する。このファイルは Design System の実際の契約を表し、プレースホルダーではない。

セマンティック token の名前と名前空間はプロジェクトが所有する。プロジェクト token 同士で DTCG alias や reference を使える。別の Design System に切り替える場合、adapter が vendor の意味体系をプロジェクトの token 値に対応付けられる。

ビルドでは `tokens/tokens.json` を CSS Custom Properties に変換し、`dist/styles/tokens.css` に書き込む。ソースに生成済みの `tokens.css` を含めてはならない。派生した token CSS は `dist/` にのみ存在する。

WDA は token を Primitive Token 層と Semantic Token 層に分けることを必須としない。スケール、再利用、階層が必要になった場合に、primitive 層や reference 層を追加できる。

## Starter の構成

WDA Minimal Starter のソースは `src/builtins/wda_minimal/` にある埋め込みアセットであり、`include_str!` によってバイナリにコンパイルされる。

- `tokens/tokens.json`：DTCG token。
- `pages/index.html`：単一ページの Starter。
- `docs/design.md`：視覚レビュー基準のパラメーター化されたソース。

テスト fixture は `tests/fixtures/wda_minimal_starter/` にある。上記三つのファイルは埋め込みアセットとバイト単位で同一である。fixture には `README.md`、`anchor.json`、`package.json`、`docs/architecture.md`、`docs/glossary.md` も含まれる。

fixture の `anchor.json` と `package.json` は、オフラインの Minimal プロジェクトの golden manifest である。`package.json` は生成結果とバイト単位で一致しなければならない。`anchor.json` は、`projectVersion`、`wdaVersion`、Design System の `version`、フォールバックの `reason` を除いて一致しなければならない。これらは実行時と実行環境に依存するためである。

## パラメーター化された design.md 基準のソース

`src/builtins/wda_minimal/docs/design.md` は、視覚レビュー基準のパラメーター化されたソースである。各 Design System のブロックを `{{#if_design_system "WDA Minimal"}}` と `{{#if_design_system "Spectrum 2"}}` で示す。レンダリング時には条件に一致するブロックだけを残す。

`## Attribution` と `## 10. Craft Floor` は単一の Design System に属さない。条件ブロックの外にあるため、どのレンダリング結果にも残る。

第 10 節「Craft Floor」は最後の節で、視覚品質レビューの基準を 11 項目含む。WDA Minimal と Spectrum 2 で第 10 節の内容は同じである。Designer は各基準を変更または削除し、その変更を同節に記録できる。これらの基準は `wda check` の必須検査ではない。

WDA は impeccable のテキストの一部だけを選び、書き換えている。その Skill、コマンド、依存関係は取り込まない。この Apache-2.0 のテキストスナップショットは上流の更新を追跡しない。`NOTICE` に出典と正確な commit を記録する。

基準には二つの置換マーカーがある。`{{DESIGN_SYSTEM_NAME}}` は Design System 名に、`{{ACCESSIBILITY_BASELINE}}` はアクセシビリティ基準に置き換わる。レンダリング後に未展開の `{{` または `}}` マーカーを残してはならない。

`src/builtins/wda_minimal.rs` の `render_criteria` が基準をレンダリングする。`src/init/documents.rs` はその結果を Generated Project の `docs/design.md` に書き込む。

## Markup とコンポーネントの境界

レイアウト、テキスト、画像、ボタン、フォーム、静的コンテンツには、ネイティブなセマンティック HTML と CSS を優先する。再利用可能な動作、製品の意味、カプセル化が必要な場合に Web Components を使う。WDA の二つの Design System 経路はいずれも Lit を採用する。Lit は Custom Element を記述する標準である。

標準的な LitElement クラス宣言と `customElements.define` による登録は、どちらもこの標準を満たす。この標準では decorator のコンパイルフラグを使わない。WDA Minimal は vendor コンポーネントライブラリを含まず、スタータースクリプト（`scripts/main.ts`）も生成しない。

Lit は Web プラットフォームを軽量に包むもので、プロジェクトを特定のフロントエンドフレームワークに結び付けない。プロジェクトへの Lit の追加と削除は、外部のパッケージマネージャーで行う。例として `npm install lit` や `bun add lit` を使う。WDA には `wda deps` コマンドはない。Minimal プロジェクトで後から Lit が必要になった場合は、ユーザーの同意を得たうえで WDA Skill が必要なパッケージを追加できる。Litだけを追加する場合、Design Systemとフォールバックの記録を維持する。Spectrumの統合では、承認された範囲で外部マネージャーを使い、[Design System and Tokens](../../design-system.md)の検証、更新、復元手順に従う。

WDA Minimal は vendor コンポーネントライブラリや専用ランタイムを必要としない。別の Design System に切り替える場合は、その vendor が公開する要素名や API を使える。WDA は vendor 名を隠す汎用 wrapper を作らない。

`<wda-include src="..."></wda-include>` は WDA 唯一の独自ビルド時構成プリミティブである。Custom Element の命名規則に従ってビルド時に展開される。ランタイムの Web Component ではなく、`dist/` にも残らない。

WDA Minimal は template language を定義しない。`<slot>`、条件、ループ、変数、式、データバインディングはない。

## 利用範囲

この文書は Core における WDA Minimal の完全な定義である。Generated Project の `docs/design.md` には、そのプロジェクトが実際に採用する token とプロジェクト固有のレビュー基準だけを記録する。この WDA レベルの定義は複製しない。

WDA Minimal は再利用可能な基準および参照である。「HTML の変更がゼロ」「再利用率 100%」「自動移行」を保証しない。移行の目標は、プロジェクトのセマンティックな構造と選択の自由を保ちながらコストを下げることである。
