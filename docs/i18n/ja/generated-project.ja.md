---
title: "生成プロジェクト"
status: stable
updated: 2026-11-12
type: Architecture
description: "wda initが生成するプロジェクトのベースライン、言語の役割、ディレクトリ、anchor.jsonとpackage.jsonの契約ファイル、dist/への対応、文書セットを定義する。"
tags: []
source: docs/generated-project.md
lang: ja
source_sha256: e2e90a5e954d9b1e2c05db3764124ab0254b55f8291815a509f6e903c5159e29
translated_at: 2026-11-12
---

# 生成プロジェクト

`wda check`と`wda build`は、`<wda-include src="...">`をそのタグを含むファイルからの相対パスとして解決する。Includeはプロジェクトのルート内に収め、循環させてはならない。両コマンドは存在しない対象、ルート外への参照、循環の参照チェーン全体を報告し、展開後にIDを検査する。診断位置は元のソースを指し、行番号とUTF-8バイト単位の列番号は1から始まる。CRLFは1つの改行として扱う。`wda check`は`dist/`に書き込まない。

`wda init`が生成するプロジェクトを、生成プロジェクト（Generated Project）と呼ぶ。これは標準ウェブプロジェクト（Standard Web Project、定義は`docs/glossary.md`）である。本仕様は、プロジェクトの技術ベースライン、各ソース種別の役割、ディレクトリ構造、`anchor.json`と`package.json`の契約ファイル、外部lockfileの責任、ソースと`dist/`の対応規則、対象範囲、文書セットを定義する。コマンドの動作と検証規則は`docs/commands.md`を参照する。

## ソースのベースラインとデプロイのベースライン

| ベースライン | 内容 |
| :--- | :--- |
| ソース | HTML5とセマンティックHTML、Modern CSS、TypeScript、およびネイティブのブラウザ機能。コンポーネントのカプセル化が本当に必要な場合にのみWeb Componentsを使う |
| デプロイ | HTML、CSS、JavaScript、およびアセット |

フロントエンドフレームワークは既定の選択肢ではない。フレームワークを導入するかどうかは、アプリケーションの実際の複雑さで決める。ルーティングの必要性だけでは、導入の十分な理由にはならない。WDAは将来の移行コストを下げることを約束するが、自動移行、無損失移行、ゼロコスト移行を約束するものではない。

## HTML、CSS、TypeScriptの役割

- HTMLは構造と静的コンテンツを担当する。
- CSSは視覚的な表現、レイアウト、レスポンシブデザイン、およびCSSだけで扱える状態を担当する。
- TypeScriptは対話的な振る舞い、必要な場合の状態管理、およびブラウザAPIの呼び出しを担当する。

TypeScriptには実際の静的型チェックが必要である。バンドラーのトランスパイル工程を型チェックとみなしてはならない。

## プロジェクトのディレクトリ構造

### `wda init`が作成する構造

`wda init`は次のパスだけを作成する。

```text
プロジェクトルート/
├── .git/                 Gitが利用可能な場合のローカルGitリポジトリ
├── .gitignore            内容は /dist/ に固定
├── README.md
├── anchor.json           WDAプロジェクト契約
├── package.json          ウェブサイト依存関係の宣言
├── docs/
│   ├── architecture.md
│   ├── design.md
│   └── glossary.md
├── pages/
│   └── index.html
└── tokens/
    └── tokens.json
```

この構造は`WDA Minimal`の基本構造である。オフラインでの初期化と、依存関係の解決に失敗したときに使われる。オンラインの初期化が成功した場合、作成される内容はDesign Systemと依存関係のインストール結果によって変わる。詳細は`docs/commands.md`を参照する。

生成プロジェクトのウェブサイト依存関係は、`package.json`だけに宣言する。WDAは`deno.jsonc`、`deno.json`、`deno.lock`を作成しない。既存の旧ファイルは必須マニフェストの代わりにはならない。

### 必要な場合にだけ作成するパス

次のパスは、プロジェクトが必要とする場合にだけ作成する。`wda init`はプレースホルダーのディレクトリを作成しない。

```text
components/            再利用できる静的なHTMLの断片、または実際に再利用するプロダクト意味のコンポーネント
styles/                共有のソーススタイルシート。単一のスタイルシートでは足りない場合に使う
scripts/               TypeScriptの対話モジュール。振る舞いのロジックをソース上で整理する必要がある場合に使う
assets/                画像、フォント、アイコンなど、プロジェクトが所有するアセット
pages/<name>.html      追加のMPAページの入口
docs/development.md    4つのWDAコマンドと必須文書では扱われない、プロジェクト固有のワークフロー
```

## プロジェクトの契約ファイル

生成プロジェクトのルートディレクトリには、契約ファイルが2つある。どちらも必須である。

| ファイル | 役割 |
| :--- | :--- |
| `anchor.json` | WDAプロジェクト契約。プロジェクトの識別情報、技術ベースライン、使用中のDesign System、初期化のフォールバック記録を定義する。`schemas/anchor.schema.json`で検証する |
| `package.json` | 唯一のウェブサイト依存関係の宣言。WDAのメタデータは含まない |

どちらかのファイルがない場合、`wda check`と`wda build`はそのファイルに対して名前付きのErrorを報告する。欠けているファイルを別のファイルから補うことはなく、旧形式の`wda.json`を代わりに読むこともない。2つのファイルがそろわないプロジェクトはビルドできない。WDAは旧形式のプロジェクトを自動変換しない。ビルドが成功すると、`anchor.json` の `wdaVersion` を更新し、`dist/` に書き込む。検証と失敗したビルドでは `wdaVersion` を変更しない。現在の形式を使うには、`wda init`で新しいプロジェクトを作成するか、2つのファイルを手作業で書き直す。

## `anchor.json`

`anchor.json`は、プロジェクト契約の中核である。プロジェクトの識別情報、技術ベースライン、Design Systemを定義する。`wda check`と`wda build`は、どちらもこのファイルを最初に読む。

### スキーマが厳格である理由

`anchor.json`は明示的に定義されたフィールドだけを許可する。人、AI、Coreのいずれもファイルの内容を理解できるようにするためである。独自の内容を追加すると、ほかの読み手はその意味や目的を判断できなくなる。

- `metadata`フィールド、独自の拡張領域、任意のトップレベルフィールドは存在しない。
- 未知のフィールドは常にWarningではなくErrorである。
- ファイルの内容に`$schema`フィールドは含まない。
- `projectFormatVersion`、`deploymentDirectory`、`projectDescription`、`wda.json`は古い名前または契約に含まれない名前であり、使わない。

### フィールド

| フィールド | 内容 | 書き込みのタイミングと変更できる主体 |
| :--- | :--- | :--- |
| `projectName` | 既定値は`Web Design Anchor Project`。ページタイトルの組み立てに使う | `init`が既定値を書き込み、その後はプロジェクトが変更する |
| `projectVersion` | `init`の実行時点のホストのローカル時刻。秒とUTCオフセットを含み、ミリ秒は含まない。その後は自動的に増えない | `init`だけが一度だけ書き込む |
| `wdaVersion` | 直近に成功した`init`または`build`で使われたWDAのバージョン | `init`が書き込み、成功したすべての`build`が更新する |
| `pageArchitecture` | 許可される値は`mpa`（マルチページアーキテクチャ）と`spa`（シングルページアーキテクチャ）。既定値は`mpa` | `init`が既定値を書き込み、その後はプロジェクトが変更する |
| `browserBaseline` | 唯一許可される値は`baseline-widely-available`（Baseline Widely Available）で、既定値でもある | `init`だけが書き込む |
| `accessibilityBaseline` | 唯一許可される値は`wcag-2.2-aa`（WCAG 2.2 AA）で、既定値でもある | `init`だけが書き込む |
| `designSystem` | 必須フィールドの`name`と`version`を持つオブジェクト。既定値は最新の安定版Spectrum。オンラインの解決に成功した場合、`version`は正確なsemverのバージョン番号になる。`WDA Minimal`へのフォールバックが適用された場合、`version`はそのWDAリリースに組み込まれたバージョンになる | `init`が書き込む。Spectrumの統合と検証が成功した後、Skillは実際のシステムに更新する。Litのみの追加では変更しない |
| `initialization.fallback` | 任意。`wda init`が、意図したDesign Systemの代わりに`WDA Minimal`を作成した場合にだけ存在する。`requestedDesignSystem`（initが最初に試したDesign System）と`reason`（フォールバックの理由を述べる空でないテキスト）を保持する。フォールバックが起きなかった場合は存在しない | `init`が書き込む。Spectrumの統合と検証が成功した後、Skillは `initialization` の外側のオブジェクトを削除する。Litのみの追加、統合の拒否または失敗では変更しない |

プロジェクトがあるフィールドを変更できるかどうかは、そのフィールドの安定性レイヤーによって決まる。詳細は`docs/architecture.md`の「Stability Layers」を参照する。

### スキーマの保存方法

正式なスキーマは`schemas/anchor.schema.json`であり、バージョン管理下に置く。バイナリは同じ内容を直接埋め込み、自動生成されたコピーはリポジトリにコミットしない。バイナリは、JSON Schemaでは表現できないフィールド間の規則も検証する。また、スキーマがすでに宣言しているフィールドを再定義しない。製品には以前の`wda.json`形式のスキーマは存在せず、検証器もその形式を読まない。

### wdaVersionの更新方法

`wdaVersion`は、直近にこのプロジェクトを正常に処理したWDAのバージョンを記録する。プロジェクトの形式バージョンではなく、形式の互換性を判断するために使ってはならない。形式の互換性の規則は`docs/architecture.md`の「Version Compatibility」を参照する。

| コマンドの結果 | `wdaVersion`の扱い |
| :--- | :--- |
| `wda init`が成功する | 書き込む |
| `wda build`が成功する | `anchor.json`の中で更新する |
| `wda build`が失敗する | 変更しない |
| `wda check` | 変更しない |

## `package.json`と外部lockfile

`package.json`は、唯一のウェブサイト依存関係の宣言である。変更は外部パッケージマネージャーのnpmまたはBunが管理する。WDAはこれを読み取り、`anchor.json`に依存関係を重複させない。

| フィールド | 内容 | 書き込む主体 |
| :--- | :--- | :--- |
| `dependencies` | 任意のオブジェクト。各値は文字列のバージョン。ウェブサイトの依存関係がないプロジェクトは、省略するか空にしてもよい | インストール後に外部マネージャーが書き込む。`init`は、オンラインの解決に成功した場合にだけSpectrumとLitの項目を書き込む |
| `packageManager` | 任意。マネージャー名、`@`、正確なバージョンの順で記述する（例：`npm@10.9.2`）。Spectrumのinitが成功した場合、利用可能なマネージャーが一つでも実際の名前と正確なバージョンを記録する。初期のMinimalには含めない | Spectrumのインストール成功時に `init` が書き込む |
| `name`、`private`、`type` | `init`がプロジェクトのパッケージ識別情報として書き込む。WDAはその値を検査しない | `init`が書き込み、プロジェクトが変更してもよい |

`package.json`のルートはオブジェクトでなければならない。それ以外の形状、文字列でない依存関係の値、または上記の規則に従わない`packageManager`の値は、WDAが名前付きのErrorとして報告する。

Manager Lockfileは、外部パッケージマネージャーが書き込むlockfileである。たとえばnpmの`package-lock.json`やBunの`bun.lock`がこれにあたる。管理はマネージャーが担当する。WDAはManager Lockfileを書き込まず、書き換えず、移行しない。`wda check`と`wda build`はインストール済みの内容を読むだけであり、`wda tools fetch`も`anchor.json`と`package.json`を変更しない。

## `dist/`

### dist/のディレクトリ構造

`dist/`は、唯一のデプロイ成果物のディレクトリである。次の例は、ビルド後のディレクトリ構造を示す。

```text
dist/
├── index.html
├── about.html
├── styles/
│   ├── main.css
│   └── tokens.css
├── scripts/
│   └── main.js
└── assets/
    └── images/
        └── hero.png
```

### ソースから dist/への対応規則

ソースから`dist/`へのパスの対応は固定されており、設定の選択肢はない。

| ソースのパス | `dist/`のパス | 規則 |
| :--- | :--- | :--- |
| `pages/<rel>.html` | `dist/<rel>.html` | 拡張子を保つ。入れ子の`index.html`ディレクトリは作らない |
| `styles/<rel>.css` | `dist/styles/<rel>.css` | 出力へそのまま対応させる |
| `scripts/<rel>.ts` | `dist/scripts/<rel>.js` | 先に型チェックし、その後にトランスパイルする |
| `assets/<rel>` | `dist/assets/<rel>` | ページまたはスタイルシートが実際に参照するファイルだけをコピーする |
| `tokens/tokens.json` | `dist/styles/tokens.css` | ビルド中にトランスパイルして生成する |

### dist/の不変条件

- `dist/`は固定の出力ディレクトリ名であり、カスタム設定はない。
- ビルドのたびにソースから再構築し、前回のビルドの出力を残さない。
- `dist/`は、WDA、ソースコード、開発ツールのない環境に単独でデプロイして動作しなければならない。
- 派生成果物は`dist/`にだけ存在してよく、ソースへ書き戻してはならない。たとえば`tokens/tokens.json`からトランスパイルされたCSSは`dist/styles/tokens.css`としてだけ存在し、ソースに`styles/tokens.css`を含めてはならない。
- ソース側と`dist/`は、互いに独立した2つのディレクトリツリーである。1つのディレクトリツリーの中で、ファイル識別子は1か所にだけ存在してよい。

### dist/に書き込まない項目

ビルドは`components/`、`anchor.json`、`package.json`、Manager Lockfile、`README.md`、`docs/`、`.gitignore`、隠しdotfiles、`.env*`の環境ファイルを`dist/`に書き込まない。`dist/`はソースに含まれず、ビルドはそのディレクトリを走査しない。

## 約束しない事項

- **`file://`プロトコル**：WDAは、`dist/`を標準の静的HTTP(S)ホストにデプロイした場合に動作することを約束する。ローカルマシンで`file://`を通じて直接開いた場合に動作することは保証しない。`wda build`が、ES module、`fetch`、またはその他`file://`では動かないことが知られている機能を検出した場合、HTTPホストを使うよう通知する。通知が出ないことは、動作を保証するものではない。
- **クリーンURL**：WDAは、ファイル拡張子の削除を約束しない。フロントエンドのルーティングを模擬するために、入れ子の`index.html`ファイルも作らない。URLの書き換えとフォールバックのルーティングは、デプロイ先のホストが処理する。
- フレームワーク移行の制限：関連する約束は「ソースのベースラインとデプロイのベースライン」を参照する。

## プロジェクトの文書セット

本節は、生成プロジェクトの文書がどのように役割を分担するかを説明する。`wda check`が文書をどう検査するかは、`docs/commands.md`の「Rule Module: Documents」を参照する。

### README.mdが唯一の入口である

`README.md`は、生成プロジェクトで唯一の閲覧の入口である。ほかの仕様と文書は、役割に応じて`docs/`に配置する。

### 4つの必須文書

プロジェクトには4つの必須文書がある。`README.md`、`docs/architecture.md`、`docs/design.md`、`docs/glossary.md`である。

- `README.md`はプロジェクトの入口であり、プロジェクトの目的と背景を説明する。
- `docs/architecture.md`は、サイトの現在の構成、技術ベースライン、各ソースパスの役割を説明する。
- `docs/design.md`は、プロジェクトが採用するDesign Systemの設定を説明する。
- `docs/glossary.md`は、プロジェクトの用語と命名規則を説明する。

`docs/naming.md`は、サポートされる旧形式ではない。検証器は互換処理を提供しない。移行中は、プロジェクトが手作業でこのファイルを`docs/glossary.md`に改名しなければならない。

### development.mdは条件付きの文書である

`docs/development.md`は必須文書ではなく、`wda init`も作成しない。4つのWDAコマンドと4つの必須文書のいずれも扱わない特別なワークフローがある場合にだけ作成する。たとえば、プロジェクト固有の開発コマンド、ツールの前提条件、自動化の流れ、手動の受け入れ手順などである。ブラウザのベースラインとアクセシビリティのベースラインの正式な情報源は`anchor.json`である。プロジェクトはこの2つのベースラインを`docs/architecture.md`または`docs/design.md`で要約してよく、その理由でこのファイルを作成する必要はない。

### architecture.mdテンプレートの2つの節と重複排除の原則

`docs/architecture.md`は、`docs/templates/generated-project/architecture.md`から作成しなければならない。テンプレートには主要な節が2つある。`Created by wda init`は、`wda init`が実際に作成するディレクトリとファイルを列挙する。`Allowed when needed`は、任意の拡張パスと、それを作成するタイミングを列挙する。テンプレートはまた、`wda init`がプレースホルダーのディレクトリを作成しないことも説明する。アーキテクチャ文書は`README.md`の内容を繰り返さない。プロジェクトの目的と背景は`README.md`だけに記録する。
