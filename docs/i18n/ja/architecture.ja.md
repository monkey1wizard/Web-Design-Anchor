---
title: "アーキテクチャ"
status: stable
updated: 2026-11-12
type: Architecture
description: "WDAリポジトリの構成、アーキテクチャ規則、文書規則を説明する。"
tags: []
source: docs/architecture.md
lang: ja
source_sha256: 4271b45225366f247eb15f9254b703732c485cec493af95c06537ca8bc0ba446
translated_at: 2026-11-12
---

# アーキテクチャ

この文書では、WDAリポジトリの構成、責任境界、アーキテクチャ規則を説明する。コマンドの動作については `docs/commands.md` を参照する。`wda init` が生成するプロジェクトについては `docs/generated-project.md` を参照する。Design System と Skill の規則については、それぞれ `docs/design-system.md` と `docs/skill-spec.md` を参照する。この文書の規則と個別文書の規則が矛盾する場合は、個別文書を優先する。

## 中心原則

### 標準を優先

WDAはWebプラットフォーム標準を優先する。適切な標準がない場合は、単純で明示的かつ広く使われている方法を採用する。標準で要件を満たせない場合に限って独自の仕組みを追加し、その規模を最小限にする。

### 単純、明示的、単一責任

各コマンド、ファイル、規則は一つの役割を担い、理解しやすくする。異なる作業を担う新しいコマンドを追加してよい。

### 人間とAIが読める

重要な情報はDesigner、AI、Engineerが理解できる必要がある。構造化された状態を `anchor.json`、`tokens.json`、またはロックファイルに保存してよいが、文書でも主要な状態を説明する。

### WDAはツールであり、ガバナンスではない

WDAはプロジェクト所有者のガバナンス手順を決めない。ADRを扱えるが、すべての決定をADRに記録することは求めない。

### 単一のAI製品に依存しない

WDAはAIが使いやすい必要があるが、特定のAI製品、ホスト、エコシステムに依存してはならない。

## システム構成

WDAは仕様、Core、Skillの三つで構成される。三者は連携するが、互いを所有しない。

| 物理コンポーネント | 場所 / パス | 性質と説明 |
| :--- | :--- | :--- |
| Specification | `docs/` | 仕様の唯一の情報源。`README.md` はナビゲーションのみを担う。 |
| Core | `src/`、`tests/`、`schemas/` | 実行可能なツール本体。`wda_core` ライブラリと `wda` バイナリを含む単一のCargoパッケージ。 |
| Skill | `skills/wda/` | 各リリースに含まれる操作マニュアル。仕様の情報源ではない。 |

仕様が `docs/` にあるのは、仕様がコードの各バージョンより長く存続するためである。Skillとバイナリはいずれも成果物であるため、Skillは `docs/` に置かない。

Coreは位置を特定できるソース診断、共通のinclude解析、Tokenの検証と出力、テキストまたはJSONの報告を担当する。行番号とUTF-8バイト単位の列番号は1から始まる。Skillは文脈に応じたアクセシビリティ、固定CSS値とTokenの提案、および任意のToken階層レビューを担当する。レビュー自体は編集を許可しない。JSONスキーマと終了状態は[JSON Output](../../commands.md#json-output)を参照する。レビュー条件と同意は[Skill Specification](../../skill-spec.md)を参照する。

## 安定性の階層

WDAの規則は五つの階層に分かれる。階層によって、変更方法、決定者、変更コストが定まる。上位の階層ほど安定性が高い。

```text
Invariant              変更しない
Default                プロジェクトが anchor.json で変更できる
Tool Form              WDAのメジャー再設計時のみ変更する
Tooling Choice         リリースで置き換え可能。プロジェクトへの影響はない
Implementation Detail  いつでも置き換え可能。外部からは見えない
```

### Invariant

Invariantは変更できない確約である。これを変更すると製品の中核定義が変わる。

- デプロイ成果物は標準のHTML、CSS、JavaScript、アセットである。
- WDAはWebサイトのランタイム依存関係ではない。
- デプロイ成果物は `dist/` のみである。
- ビルド結果を誤らせるErrorはビルドを停止する。Errorの定義については `docs/commands.md` のDiagnostic Outputを参照する。

### Default

プロジェクトは `anchor.json` で異なる値を明示的に設定できる。Designerが選択を明示しない場合、WDAは既定値を使い、質問しない。既定値については `docs/generated-project.md` の `anchor.json` フィールド表を参照する。

### Tool Form

Tool FormはCoreの実行方法を定める。実行方法の変更は主要なアーキテクチャ再設計にあたる。

- WDAは単独で動作するCLIである。
- ユーザーがPATHに置く必要があるのはネイティブバイナリ一つだけである。`wda init`、`wda check`、`wda build` は、PATH上にNode.js、Bun、Deno、esbuildを必要としない。ウェブサイトのパッケージを使うプロジェクトには外部のパッケージマネージャーが必要であり、`wda init` はSpectrumとLitをインストールするときだけこれを呼び出す。

### Tooling Choice

Tooling Choiceは、WDAがTool Formをどのように実装するかを定める。リリースで以下のツールを置き換えてよい。ただし、プロジェクト契約、四つのコマンドの責任、ビルド結果を変えてはならず、プロジェクト側の変更も要求してはならない。

- CLIバイナリはRustで記述する。
- Rolldown 1.3.1は、そのRust APIを通じて、インストール済みパッケージとプロジェクト自身のページおよびスクリプトからブラウザー向けのESM出力をビルドする。
- TypeScriptネイティブコンパイラーは、リリースごとに一つのバージョンを組み込む。`packaging/toolchain.json` がそのバージョン、六つのプラットフォーム向けパッケージ、`sha512` SRIを固定する。
- npmまたはBunは外部のパッケージマネージャーとして実行され、`package.json` の変更、マネージャーのロックファイル、バージョン解決、インストールを担う。

### Implementation Detail

Implementation DetailはCore内部のコード構成である。外部契約を変えずに置き換えてよい。

- Rustモジュールの分割方法。
- atomic initのロールバック実装。

## スコープ

WDAが行うのはビルドのみである。単独でデプロイできる静的ファイルを生成する。ホスティングやデプロイのサービスは提供しない。DesignerまたはAIホストは、成果物をGitHub Pages、Cloudflare Pages、Netlify、S3などの静的ホスティングサービスにデプロイできる。

組み込みホスティングはWDAをクラウドサービスに結び付け、アカウント、キー、課金、プロバイダーAPIの保守作業を増やす。この作業は標準Web成果物に影響しないため、WDAのスコープ外である。

WDAは以下の機能を恒久的に対象外とする。

- クリーンURL、URL書き換え、フォールバックルーティング。これらはデプロイ環境が担う。
- プレビューサーバー。Coreには含めない。

## プロジェクトディレクトリ構造

以下の `src/` ツリーは現在の `git ls-files src` の出力に基づく。それ以外は、現在のリポジトリにあるトップレベルのディレクトリとファイルである。

```text
web-design-anchor/
├── .github/                 CI、リリースワークフロー、GitHub向け指示
├── AGENTS.md                プロジェクトエージェント指示と権威あるチェックコマンド
├── CHANGELOG.md             バージョン変更履歴
├── examples/                サンプルプロジェクト
├── tools/writing/           文書の品質チェックツール
├── Cargo.toml               wda_coreライブラリとwdaバイナリを含む単一パッケージ
├── Cargo.lock               Rust依存関係ロックファイル
├── schemas/                 プロジェクトのマニフェストファイルのスキーマ
├── tests/                   統合テストとテストケース
├── docs/                    プロジェクト仕様と参照文書
├── skills/wda/              各リリースに含まれるSkill
├── packaging/               リリースとインストール用ファイル
├── src/                     実装ソース
├── plugin.json              Agent Pluginsの宣言
├── README.md                プロジェクト概要
├── SECURITY.md              セキュリティモデル
├── NOTICE                   第三者著作物の表示
└── LICENSE                  ライセンス条項
```

`src/builtins/spectrum_two/` はSpectrum 2のスターターページ、トークン、`scripts/main.ts` を保持する。`src/init/mod.rs` はこれらのアセットを使って初期プロジェクトを生成する。四つの検証モジュールは `src/lib.rs` が提供する共通インターフェースを介して使われ、`src/main.rs` に検証規則はない。この構成により、`wda check` と `wda build` は同じ検証エンジンを共有する。

## バージョン互換性

### 将来の形式バージョン

現行のスキーマに `schemaVersion` フィールドはない。WDAは既存プロジェクト形式との後方互換性を優先する。将来の形式に対する破壊的変更には `schemaVersion` で明示的なバージョンを定義し、`check` または `build` の実行時に暗黙適用してはならない。新しいWDAバージョンは互換性のある古い形式のプロジェクトをビルドできるが、プロジェクト構造を自動移行しない。新しい実装は、サポートを約束した古い形式向けのバリデーターと依存関係処理分岐を維持する必要がある。

### 後方互換性と暗黙の移行禁止

依存関係の宣言形式は後方互換性を保ち、WDAはプロジェクトファイルを暗黙に書き換えてはならない。

- 現行形式は `anchor.json` でWDAの設定を、`package.json` でウェブサイトの依存関係を宣言する。認識できない設定を使うプロジェクトは未対応形式であり、CoreはErrorを報告する。
- 両方のマニフェストが必須である。旧形式の `wda.json`、`deno.jsonc`、`deno.json` はどちらの代わりにもならない。Coreはマニフェストがない場合にErrorを報告し、旧形式を自動移行しない。旧ファイルは両方の現行マニフェストと共存できる。
- `wda check` はプロジェクトファイルを書き込まない。`wda build` が成功すると `anchor.json` の `wdaVersion` を更新し、`dist/` に書き込む。ビルドが失敗した場合、`wdaVersion` は変わらない。どちらのコマンドもプロジェクト形式を移行しない。

### 四段階の検証セマンティクス

```text
現行形式           → OK。例: プロジェクトが現行の設定を使う
サポート対象の旧形式 → Warning。例: サポート対象の旧形式を使う既存プロジェクト
非推奨の旧形式       → より明示的なWarning
未対応形式           → Error。例: 認識できない形式または二重設定
```

### 一回限りの例外 (2026-11-06)

2026-11-06、Ownerは互換性規則およびTooling Choice規則に対する一回限りの例外を決定した。現段階ではOwnerがWDAの唯一のユーザーであるため、今回の移行は後方互換性の義務から免除され、以前のバリデーターや初期の依存関係分岐を維持する必要はない。ツールチェーンの置き換えは、プロジェクト側の変更を要求する場合があり、`dist/` のビルド結果構成を変更する場合がある。また、プロジェクトへの影響はないとする規則を免除する。この例外は2026-11-06のツールチェーン移行にのみ適用され、以降の変更には適用されない。

## 依存関係モデル

Coreは以下のアーキテクチャ規則に従って依存関係を扱う。

- `package.json` がウェブサイトの依存関係の唯一の情報源であり、`anchor.json` はそれを重複して宣言しない。
- 外部のパッケージマネージャー、npmまたはBunが、`package.json` の変更、そのロックファイル、バージョン解決、推移的な依存バージョン、インストールを担う。WDAはnpmのバージョンを解決せず、CDNミラーやvendorディレクトリも保持しない。
- `wda check` と `wda build` はインストール済みのパッケージ内容だけを読み取る。パッケージをインストール、更新、書き換えることはなく、パッケージのためにネットワークを使うこともない。
- ウェブサイトのモジュールはリモートCDNから読み込まない。リモートモジュールのimportは診断として報告する。
- `wda init` がパッケージマネージャーを実行するのは、SpectrumとLitをインストールするときだけである。パッケージのインストールは第三者のコードを実行するため、`check` と `build` とは別の独立した手順とする。
- WDAは外部のパッケージマネージャーのライフサイクルスクリプトの動作を明記するが、その副作用を元に戻せるとは約束しない。
- 読み取り専用コマンドは依存関係宣言を移行または書き換えてはならない。

各コマンドの依存関係動作については `docs/commands.md` を参照する。

## Design Systemの境界

既定のDesign Systemは最新の安定版Spectrumである。ネットワークが利用可能で使用可能なパッケージマネージャーがある場合、`wda init` はそのマネージャーを通じてSpectrumとLitをインストールし、正確なバージョンはマネージャーが記録する。ネットワークが使えない場合、使用可能なパッケージマネージャーがない場合、または解決やインストールに失敗した場合、`wda init` は組み込みの `WDA Minimal` を生成し、その理由を説明する。ローカルのパッケージキャッシュがあっても、initがSpectrumに切り替わることはない。`anchor.json` は実際のDesign Systemを記録し、フォールバック時には元の予定だったシステムと理由も記録する。トークンの正本は `tokens/tokens.json` であり、派生CSSは `dist/` にのみ存在する。

Design Systemの名前と識別情報を使用できるのは、解決シーム `src/init/` と組み込みアセット層 `src/builtins/` のみである。それ以外のCoreコードはDesign Systemの識別情報で分岐してはならない。唯一の許可リストは `tests/neutrality.rs` の `IDENTITY_SEAM_FILES` である。詳細は `docs/design-system.md` を参照する。

## Skillの境界

SkillはAI用の操作マニュアルである。仕様の情報源でも、二つ目のジェネレーターでもない。Coreと `anchor.json` 契約は、SkillだけがWDAを操作するとは仮定してはならない。WDAが提供するSkillは `skills/wda/` にある `wda` 一つだけであり、所属するリリースと同じバージョンを使う。Skillはホストがネイティブバイナリを実行し、stdout、stderr、終了ステータスを読み取れることを前提とする。詳細は `docs/skill-spec.md` を参照する。

## Webプレビュー

### Coreはプレビューを提供しない

プレビューはAIホストが担う。AIホストはWebView、Artifact、外部ブラウザーを使って `dist/` を表示できる。CLIはWebプレビューを担当せず、プレビューサーバーも含まない。

プレビューサーバーは常駐プロセス、ポート、HTTPサーバーの保守作業を増やし、単独CLIの役割に合わない。AIホスト間で動作するプレビュー機能が必要になった場合は、プラグインまたはデザインツール連携が提供するべきである。

### 三つの基本規則

すべてのAIホストがWebページを表示できるとは限らない。以下の原則により、Designerが変更を見ずに視覚的な結果を承認することを防ぐ。

1. `wda build` の成功後、AIホストが成果物を取得できるように `dist/` の絶対パスを出力する必要がある。
2. AIホストに組み込みプレビューがない場合、プレビューに制限があることを伝え、視覚的な変更を完了と宣言してはならない。
3. Designerが確認していない変更を受け入れ済みとして扱ってはならない。

### 可逆性の基本

誰も見ていない変更は受け入れ済みとみなさない。そのため、以前に受け入れた状態に戻すとき、Designerが見ていないバージョンに戻ってはならない。

Gitが利用可能な場合、`wda init` が作成したローカルGitリポジトリの最初のコミット、またはユーザーが後でGitをインストールして作成した最初のコミットが復元の基準となる。Gitがない場合、復元の基準は存在せず、AIホストのチェックポイント、ファイルスナップショット、手動復元は補助としてのみ使える。

## 開発と検証

`AGENTS.md` の権威あるチェック境界は、次の二つのコマンドを指定する。

```sh
cargo test
deno run --cached-only --frozen --allow-read --allow-env --allow-sys --config tools/writing/deno.json tools/writing/check.ts --required
```

ライティングチェッカーにはDenoが必要であり、権威ある文書チェックはDenoの下で実行する。製品の `wda init`、`wda check`、`wda build` にはDenoは不要である。`#[ignore]` が付いたテストは通常の実行では動かない。これには、実際のパッケージマネージャーまたは実際のネットワークを必要とするプローブが含まれ、その結果だけでリリースをゲートすることはない。

必須文書用のライティングチェッカーは `tools/writing/` にある。初回利用前に `AGENTS.md` の指示に従い依存関係をインストールする。

## このリポジトリの文書規則

この節のライティング規則は必須文書セットに適用する。この節のその他の規則は、このリポジトリの `docs/` にのみ適用する。`wda init` が生成するプロジェクトの文書規則は `wda check` が検証する。詳細は `docs/commands.md` のRule Module: Documentsを参照する。

### ライティング規則

文書では正確さ、明瞭さ、平易さの順に優先する。要件が衝突する場合は、まず正確さを保ち、次に明瞭さを保つ。

必須文書セットには次のパスが含まれる。

- en-US: `README.md`、`SECURITY.md`、`docs/*.md`、`docs/adr/*.md`、`docs/design-systems/*.md`。
- en-US: `skills/wda/SKILL.md`、`skills/wda/references/*.md`。
- en-US: `docs/templates/generated-project/*.md`、`CHANGELOG.md`。
- en-US: `src/builtins/wda_minimal/docs/design.md`。
- zh-TW: `docs/i18n/zh-TW/*.zh-TW.md`、`AGENTS.md`、`.dev/project.md`。
- ja: `docs/i18n/ja/*.ja.md`。

チェッカーは `tools/writing/` にある。初回インストール時はリポジトリルートから次のコマンドを実行する。

```sh
deno install --config tools/writing/deno.json
```

必須文書を検査するには、リポジトリルートから次のコマンドを実行する。

```sh
deno run --cached-only --frozen --allow-read --allow-env --allow-sys --config tools/writing/deno.json tools/writing/check.ts --required
```

`prh` と `no-unmatched-pair` の検出は、zh-TWとjaのどちらでもハードエラーである。zh-TWでは `sentence-length` と `write-good` の検出は助言扱いである。jaでは `sentence-length` の検出は助言扱いである。zh-TW文書を修正する場合、`zhtw-mcp` は助言のみを行い、権威ある検査ゲートではない。プロジェクト指定用語と `zhtw-mcp` の提案が衝突する場合は、プロジェクト指定用語を優先する。

すべての修正で事実、数値、条件、範囲、不確実性を保持する必要がある。文書を修正するときは編集前後の不変条件を確認する。必要な編集が意味を変えてしまう場合は、停止して報告する。

ADRのStatus History節とその内容を書き換えてはならない。チェッカーは引き続きその節を検査するが、検出事項は助言としてのみ表示する。

### 多言語ポリシー

このリポジトリの正本言語は英語である。英語の正本ファイルは、`README.md`、`SECURITY.md`、六つの契約ファイル `docs/architecture.md`、`docs/commands.md`、`docs/design-system.md`、`docs/generated-project.md`、`docs/glossary.md`、`docs/skill-spec.md`、`docs/design-systems/` の二つのファイル、`docs/adr/` の六つのADR、`skills/wda/SKILL.md` と `skills/wda/references/*.md` である。`docs/templates/generated-project/` のファイルは従来どおりen-USのままとする。

翻訳は `docs/i18n/<lang>/<name>.<lang>.md` に置く。zh-TWの集合は閉じており、次の十ファイルのみで構成する。

- `docs/i18n/zh-TW/README.zh-TW.md`
- `docs/i18n/zh-TW/SECURITY.zh-TW.md`
- `docs/i18n/zh-TW/architecture.zh-TW.md`
- `docs/i18n/zh-TW/commands.zh-TW.md`
- `docs/i18n/zh-TW/design-system.zh-TW.md`
- `docs/i18n/zh-TW/generated-project.zh-TW.md`
- `docs/i18n/zh-TW/glossary.zh-TW.md`
- `docs/i18n/zh-TW/skill-spec.zh-TW.md`
- `docs/i18n/zh-TW/spectrum-adapter.zh-TW.md`
- `docs/i18n/zh-TW/wda-minimal.zh-TW.md`

jaの集合も閉じており、次の十ファイルのみで構成する。

- `docs/i18n/ja/README.ja.md`
- `docs/i18n/ja/SECURITY.ja.md`
- `docs/i18n/ja/architecture.ja.md`
- `docs/i18n/ja/commands.ja.md`
- `docs/i18n/ja/design-system.ja.md`
- `docs/i18n/ja/generated-project.ja.md`
- `docs/i18n/ja/glossary.ja.md`
- `docs/i18n/ja/skill-spec.ja.md`
- `docs/i18n/ja/spectrum-adapter.ja.md`
- `docs/i18n/ja/wda-minimal.ja.md`

各翻訳は六つの必須front matterキーに加え、四つの来歴キーを持つ。

| Key | 内容 |
| :--- | :--- |
| `source` | 正本ファイルのリポジトリ相対パス。例: `docs/architecture.md`。 |
| `lang` | 翻訳の言語タグ。例: `zh-TW` または `ja`。 |
| `source_sha256` | 行末をLFに正規化した正本ファイルのSHA-256。小文字の16進数を使う。 |
| `translated_at` | 翻訳日。形式は `YYYY-MM-DD`。 |

鮮度規則: `tools/writing/check.ts --required` は翻訳ファイル名から正本パスを導出する。`source_sha256` がLF正規化後の正本ファイルのSHA-256と異なる場合、検査は失敗する。`tests/docs.rs` の `EXPECTED_DOCS_MD_FILES` は `docs/` 内のファイル集合を固定する。

`docs/i18n/<lang>/<name>.<lang>.md` のタグには、記述される書記変種を識別できる最短のBCP 47タグを使う。

ADRは英語のみであり、翻訳しない。ADRのh2見出しは `## Context`、`## Options`、`## Decision`、`## Consequences`、`## Status History` である。

### 必須front matterキー

`docs/` 内のすべてのMarkdownファイルには、次の六つのキーが必要である。

| Key | 内容 |
| :--- | :--- |
| `title` | 空にしてはならない。 |
| `status` | `draft`、`stable`、`deprecated` のいずれか。 |
| `updated` | `YYYY-MM-DD` 形式。内容に意味上の変更があれば更新する。 |
| `type` | プロジェクトの語彙 `Architecture`、`Reference`、`Guide`、`ADR`、`Template`、`Checklist` を使う。 |
| `description` | 短い一文で説明する。 |
| `tags` | 配列を使う。空でもよい。 |

`type`、`description`、`tags` はopen-knowledge-format (OKF) のSPEC.mdの定義に従う。OKFは `type` を必須の自由形式文字列と定義しているため、上記の六つの値はOKFの語彙ではなく、このプロジェクトの規約である。OKFが定義する `status` の値は三つのみである。

### Statusの語彙

| 値 | 意味 |
| :--- | :--- |
| `draft` | 編集中であり、正式な根拠ではない。 |
| `stable` | 現行規則を記録する。 |
| `deprecated` | 別の文書が内容を置き換えた。履歴参照のため文書を残す。 |

### sourceRefsが必要な場合

文書が別の正本ソースから派生する場合、`sourceRefs` を設定する必要がある。各項目はリポジトリルートからのパスまたは節を使う。この規則は `README.md` とルートディレクトリの `prefile_*` ファイルには適用しない。ADRは `sourceRefs` を設定しない。理由は次の節で説明する。

### ADRは一次資料

各ADRは長期的なアーキテクチャ上の懸念を一つ記録する一次資料である。そのため `sourceRefs` を設定せず、本文で他のADR、アーキテクチャ文書、`README.md` を参照しない。参照先文書は書き換え、移動、削除される可能性があり、参照が壊れるためである。ADRで外部規則の理解が必要な場合は、その規則を要約し、逐語的に複製したり壊れやすい参照を残したりしない。

ADRは `src/` モジュール、`tests/` のフィクスチャ、ルートの `NOTICE` など、コードや物理ファイルを直接参照できる。これらのパスは抽象文書ではなく、実在するファイルを指す。

### ADRの採用基準

新しいADRは次の両方を満たす必要がある。

1. 却下した選択肢が実装済み、予定済み、または既定値として使用済みである。
2. 決定を裏付ける測定値に、具体的な数値、失敗メッセージ、または覆された前提のいずれかが記載されている。

現在の運用規則は該当する契約文書に置き、独立したADRにはしない。

### 番号とファイル名

表示番号は `ADR-NN` 形式を使う。ファイル名は `docs/adr/NN-<kebab-case-slug>.md` 形式を使う。どちらも二桁であり、番号が一桁の場合は先頭にゼロを付ける。

### 固定された五つの節

各ADRには以下の五節を順番どおりに含める。同等の選択肢がなく、決定もない場合、ADRを作成する必要はない。

| 節 | 内容 |
| :--- | :--- |
| Context | 問題の背景と当時の制約を説明する。 |
| Options | 二つ以上の選択肢を挙げ、それぞれのコストとトレードオフを説明する。 |
| Decision | 選択を明示的に記録する。 |
| Consequences | 利点、受け入れたトレードオフ、システムの他の部分への影響を説明する。 |
| Status History | 置き換えや改訂を追跡する日付と一文の注記を記録する。 |

YAML front matterの `status` フィールドは現在の状態を記録する。別のADRが置き換える場合、状態を `deprecated` にし、Status Historyに置き換え元を記録する。

統合された各ADRは、吸収したADRのすべてのStatus History項目を順序どおり、原文のまま保持し、末尾に2026-09-21付けの統合項目を追加する。統合元はfront matterの `absorbs` に記載する。本文には `ADR-<number>` を記載しない。これは単一ファイルの追記専用規則に対する一度限りの例外である。Git履歴には正確なファイルのアーカイブが残る。

2026-10-22、Ownerは二つ目の一度限りの例外を決定した。ADRのStatus History項目は、日付と意味を変えずに英語へ翻訳し、各ADRの末尾に英語へ翻訳したことと決定に変更がないことを示す項目を追加する。Git履歴には正確なファイルのアーカイブが残る。

### 主題別統合と対応表

このリポジトリでは26件の個別決定を6件の主題別ADRに統合した。各ADRは長期的なアーキテクチャ上の懸念を一つ記録する。対応は次のとおりである。

| 新番号 | 主題 | 吸収した旧決定 (absorbs) | ファイル名 |
| :--- | :--- | :--- | :--- |
| ADR-01 | 製品のスコープと出力境界 | 001, 002, 016, 017, 028 | `docs/adr/01-product-scope-and-output-boundaries.md` |
| ADR-02 | 文書作成、依存関係、ツールチェーン | 006, 011, 013 | `docs/adr/02-authoring-dependencies-and-toolchain.md` |
| ADR-03 | ビルド、検証、診断 | 007, 010, 012, 015, 018, 027 | `docs/adr/03-build-validation-and-diagnostics.md` |
| ADR-04 | Design System、トークン、中立性 | 008, 014, 019, 024, 025, 031 | `docs/adr/04-design-systems-tokens-and-neutrality.md` |
| ADR-05 | 生成プロジェクトのライフサイクルと可逆性 | 023, 029, 030 | `docs/adr/05-generated-project-lifecycle-and-reversibility.md` |
| ADR-06 | AI Skillの境界と評価 | 003, 022, 026 | `docs/adr/06-ai-skill-boundaries-and-evaluation.md` |

### 改行と文字コード

GitリポジトリはBOMなしのUTF-8とLF改行を使う。ローカル設定が `core.autocrlf=true` の場合、チェックアウト時にCRLFで表示されても、コミットがCRLFという意味ではない。編集時は既存の改行を維持する。改行変換だけを目的にファイル全体を書き換えてはならない。

## 廃止されたADR番号

以下の番号は廃止済みであり、再割り当てしない。

| 番号 | 理由 |
| :--- | :--- |
| ADR-004 | ホスティングを提供しない決定は、この文書のScopeに含まれた。 |
| ADR-005 | `wda init` が生成するプロジェクトの文書構造規則は `docs/generated-project.md` に含まれた。 |
| ADR-009 | AIホストによるプレビューと三つの基本規則は、この文書のWeb Previewに含まれた。 |
| ADR-020 | 2026-10-13に削除された。現在の配置先はない。 |
| ADR-021 | `wda init` が生成するプロジェクトのアーキテクチャ文書テンプレート規則は `docs/generated-project.md` と `docs/templates/generated-project/` に含まれた。 |
