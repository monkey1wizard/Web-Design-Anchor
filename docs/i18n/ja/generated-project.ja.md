---
title: "生成プロジェクト"
status: stable
updated: 2026-10-23
type: Architecture
description: "wda initが生成するプロジェクトのベースライン、言語の役割、ディレクトリ、契約フィールド、dist/への対応、文書セットを定義する。"
tags: []
source: docs/generated-project.md
lang: ja
source_sha256: 72a796866e004232ab2f83acc6012d60f0766e0fa8fd077a26c3ac83a41053e7
translated_at: 2026-10-23
---

# 生成プロジェクト

`wda init`が生成するプロジェクトを生成プロジェクトと呼ぶ。これは標準Webプロジェクトである（`docs/glossary.md`で定義）。この仕様では技術的なベースライン、各ソース形式の役割、ディレクトリ構造、`wda.json`のフィールド、ソースから`dist/`への対応規則、サポート範囲、文書セットを説明する。コマンドの動作と検証規則は`docs/commands.md`を参照する。

## ソースとデプロイのベースライン

| ベースライン | 内容 |
| :--- | :--- |
| ソース | HTML5とセマンティックHTML、モダンCSS、TypeScript、ブラウザーのネイティブ機能。コンポーネントのカプセル化が本当に必要な場合はWeb Componentsを使う |
| デプロイ | HTML、CSS、JavaScript、アセット |

フロントエンドフレームワークは既定の選択肢ではない。導入するかどうかはアプリケーションの実際の複雑さで判断する。ルーティングの必要性だけでは導入理由として不十分である。WDAは後の移行コストを下げることを約束する。自動移行、損失のない移行、またはコストゼロの移行を約束するものではない。

## HTML、CSS、TypeScriptの役割

- HTMLは構造と静的コンテンツを担う。
- CSSは視覚表現、レイアウト、レスポンシブデザイン、CSSだけで扱える状態を担う。
- TypeScriptは対話的な動作、必要な状態管理、ブラウザーAPI呼び出しを担う。

TypeScriptには実際の静的型検査が必要である。バンドラーの変換処理を型検査として扱ってはならない。

## プロジェクトディレクトリツリー

### wda initが作成するツリー

`wda init`が作成するパスは次のものだけである。

```text
project root/
├── .git/                 ローカルGitリポジトリ
├── .gitignore            内容は /dist/ に固定
├── README.md
├── wda.json
├── docs/
│   ├── architecture.md
│   ├── design.md
│   └── glossary.md
├── pages/
│   └── index.html
└── tokens/
    └── tokens.json
```

このディレクトリ構造は`WDA Minimal`の基本構造である。オフライン初期化時と依存関係の解決に失敗した場合に使う。オンライン初期化が成功した場合、作成される内容はDesign Systemと依存関係のインストール結果によって異なる。詳細は`docs/commands.md`を参照する。

生成プロジェクトの依存関係宣言ファイルは`deno.jsonc`のみである。パッケージは`imports`フィールドに記載し、外部依存関係は`deno.lock`で管理する。`deno.json`はサポートしない。`wda build`が使う厳格な`deno.json`は、プロジェクト外の一時ディレクトリにのみ配置する。

### 必要な場合に作成するディレクトリ

次のパスはプロジェクトで必要になった場合にのみ作成する。`wda init`はプレースホルダーのディレクトリを作成しない。

```text
components/            再利用する静的HTML断片、または実際に再利用する製品意味論を持つコンポーネント
styles/                単一のスタイルシートでは足りない場合の共有ソーススタイルシート
scripts/               動作ロジックを整理する必要がある場合のTypeScriptモジュール
assets/                画像、フォント、アイコンなどプロジェクト所有のアセット
pages/<name>.html      追加のMPAページエントリポイント
docs/development.md    WDAの四つのコマンドと必須文書で扱わないプロジェクト固有の作業手順
```

## `wda.json`

すべての生成プロジェクトはルートに`wda.json`を持つ。このファイルはプロジェクト契約の中核であり、プロジェクト識別情報、技術的ベースライン、Design Systemを定義する。`wda check`と`wda build`は、最初にこのファイルを読む。

### スキーマを厳格にする理由

`wda.json`で許可するのは明示的に定義されたフィールドのみである。人間、AI、Coreのすべてがファイルを理解できるようにするためである。独自の内容を追加すると、他の読み手がその意味や目的を判断できなくなる。

- `metadata`フィールド、独自拡張領域、任意のトップレベルフィールドはない。
- 未知のフィールドは常にErrorであり、Warningではない。
- ファイルに`$schema`フィールドは含めない。
- `projectFormatVersion`、`deploymentDirectory`、`projectDescription`は古い名前または契約外の名前であり、使わない。

### フィールド

| フィールド | 内容 | 書き込み時期と変更できる主体 |
| :--- | :--- | :--- |
| `projectName` | 既定値は`Web Design Anchor Project`。ページタイトルの構成に使う | `init`が既定値を書き、その後プロジェクトが変更する |
| `projectVersion` | `init`実行時のホストの現地時刻。秒とUTCオフセットを含み、ミリ秒は含まない。その後自動加算しない | `init`のみが一度書き込む |
| `wdaVersion` | このプロジェクトを最後に正常処理した`init`または`build`のWDAバージョン | `init`が書き込み、成功した`build`ごとに更新する |
| `pageArchitecture` | 許可値は複数ページ構成の`mpa`と単一ページ構成の`spa`。既定値は`mpa` | `init`が既定値を書き、その後プロジェクトが変更する |
| `browserBaseline` | 許可値は`baseline-widely-available`（Baseline Widely Available）のみで、これが既定値 | `init`のみが書き込む |
| `accessibilityBaseline` | 許可値は`wcag-2.2-aa`（WCAG 2.2 AA）のみで、これが既定値 | `init`のみが書き込む |
| `designSystem` | 既定値は最新の安定版Spectrum。オンライン解決成功時の`version`は正確なsemverバージョン。`WDA Minimal`にフォールバックする場合は、そのWDAリリースに組み込まれたバージョン | `init`のみが書き込む |

プロジェクトがフィールドを変更できるかどうかは、その安定性階層による。詳細は`docs/architecture.md`の「安定性の階層」を参照する。

### スキーマの保存方法

正規スキーマはバージョン管理下の`schemas/wda.schema.json`である。バイナリは同じ内容を直接埋め込み、自動生成したコピーをリポジトリにコミットしない。JSON Schemaで表現できないフィールド間規則もバイナリが検証する。スキーマがすでに宣言するフィールドを再定義しない。旧形式をサポートする場合に限り、レガシースキーマを追加する。

### wdaVersionの更新

`wdaVersion`は、このプロジェクトを最後に正常処理したWDAのバージョンを記録する。プロジェクト形式のバージョンではない。形式互換性の判定にこのフィールドを使わない。互換性規則は`docs/architecture.md`の「バージョン互換性」を参照する。

| コマンド結果 | `wdaVersion`の扱い |
| :--- | :--- |
| `wda init`成功 | 書き込む |
| `wda build`成功 | 更新する |
| `wda build`失敗 | 変更しない |
| `wda check` | 変更しない |
| `wda deps` | 変更しない |

## `dist/`

### dist/ディレクトリツリー

`dist/`はデプロイ成果物を置く唯一のディレクトリである。ビルド後の構造例を示す。

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

### ソースからdist/への対応規則

ソースから`dist/`へのパス対応は固定であり、設定で変更できない。

| ソースパス | `dist/`パス | 規則 |
| :--- | :--- | :--- |
| `pages/<rel>.html` | `dist/<rel>.html` | 拡張子を維持する。入れ子の`index.html`ディレクトリは作成しない |
| `styles/<rel>.css` | `dist/styles/<rel>.css` | 出力へ直接対応付ける |
| `scripts/<rel>.ts` | `dist/scripts/<rel>.js` | 最初に型検査し、その後変換する |
| `assets/<rel>` | `dist/assets/<rel>` | ページまたはスタイルシートが実際に参照するファイルだけをコピーする |
| `tokens/tokens.json` | `dist/styles/tokens.css` | ビルド中に変換して生成する |

### dist/の不変条件

- `dist/`は固定の出力ディレクトリ名であり、独自設定はない。
- ビルドのたびにソースから再構築し、前回の出力を残さない。
- `dist/`はWDA、ソースコード、開発ツールのない環境に単独でデプロイして実行できなければならない。
- 生成物は`dist/`にのみ存在し、ソースへ書き戻してはならない。たとえば、`tokens/tokens.json`から変換したCSSは`dist/styles/tokens.css`にのみ存在し、ソースに`styles/tokens.css`を置かない。
- ソース側と`dist/`は独立した二つのディレクトリツリーである。同じツリー内で、ファイル識別子を複数の場所に置かない。

### dist/へ出力しない項目

ビルドは`components/`、`wda.json`、`README.md`、`docs/`、`.gitignore`、隠しdotfile、`.env*`環境ファイルを`dist/`へ出力しない。`dist/`はソースの一部ではなく、ビルドはこのディレクトリを走査しない。

## 非保証事項

- **`file://`プロトコル**: WDAは、標準的な静的HTTP(S)ホストへのデプロイで`dist/`が動作することを保証する。ローカルマシンで`file://`から直接開いた場合の動作は保証しない。`wda build`がES module、`fetch`、その他`file://`で動かない既知の機能を検出した場合は、HTTPホストを使うよう注意を促す。注意が表示されなかったことは、動作保証を意味しない。
- **クリーンURL**: WDAはファイル拡張子の削除を保証せず、フロントエンドルーティングを模倣する入れ子の`index.html`も作らない。URL書き換えとフォールバックルーティングはデプロイ先が扱う。
- **フレームワーク移行の制限**: 関連する保証については「ソースとデプロイのベースライン」を参照する。

## プロジェクト文書セット

この節では生成プロジェクトの文書の役割分担を説明する。`wda check`による文書検査は`docs/commands.md`の「ルールモジュール: 文書」を参照する。

### README.mdは唯一の入口

`README.md`は生成プロジェクトで読む唯一の入口である。それ以外の仕様と文書は役割に応じて`docs/`に置く。

### 必須文書は四つ

プロジェクトに必要な文書は`README.md`、`docs/architecture.md`、`docs/design.md`、`docs/glossary.md`の四つである。

- `README.md`はプロジェクトの入口であり、目的と背景を説明する。
- `docs/architecture.md`はサイトの現在の構成、技術的ベースライン、各ソースパスの役割を説明する。
- `docs/design.md`はプロジェクトが採用するDesign Systemの設定を説明する。
- `docs/glossary.md`はプロジェクトの用語と命名規則を説明する。

`docs/naming.md`はサポート対象の旧形式ではなく、バリデーターも互換処理を行わない。移行時には手作業で`docs/glossary.md`へファイル名を変更する。

### development.mdは条件付き文書

`docs/development.md`は必須ではなく、`wda init`も作成しない。WDAの四つのコマンドと必須文書では扱わない特別な作業手順がある場合にのみ作成する。たとえば、プロジェクト固有の開発コマンド、ツールの前提条件、自動化フロー、手動の受け入れ手順を記録する。ブラウザーとアクセシビリティのベースラインの権威ある情報源は`wda.json`である。プロジェクトはその二つを`docs/architecture.md`または`docs/design.md`に要約してよく、そのためだけにこのファイルを作成する必要はない。

### architecture.mdテンプレートの二つの節と重複ゼロの原則

`docs/architecture.md`は`docs/templates/generated-project/architecture.md`を基に作成する。テンプレートには二つの主な節がある。`Created by wda init`には`wda init`が実際に作成するディレクトリとファイルを記載する。`Allowed when needed`には任意の拡張パスと作成条件を記載する。テンプレートは、`wda init`がプレースホルダーディレクトリを作らないことも説明する。アーキテクチャ文書で`README.md`の内容を繰り返さない。プロジェクトの目的と背景は`README.md`だけに記録する。
