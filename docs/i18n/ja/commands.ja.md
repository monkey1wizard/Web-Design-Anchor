---
title: "コマンド"
status: stable
updated: 2026-11-12
type: Reference
description: "コマンドのツール要件、動作、診断、終了ステータスを説明する。"
tags: []
source: docs/commands.md
lang: ja
source_sha256: 64e2d337052b0313bc4c6f96438886cf0d677f9cd96cf316eb600759cf5ee777
translated_at: 2026-11-12
---

# コマンド

Coreは4つのコマンドを提供する。DesignerとAIはプロジェクトファイルを直接編集し、Coreは一般的な編集を行わない。Designerのワークフロー、Skillのインストール、プレビューによってコマンドが増えることはない。

```text
wda init    Create a new project and, when Git is usable, a local Git repository
wda check   Validate without producing output
wda build   Validate, type-check, then build the output in dist/ if validation passes
wda tools fetch   Restore the embedded TypeScript tool cache for this platform
```

`wda --version`はバージョン情報を表示するだけであり、Skillがバージョンを比較するために使う。5つ目のコマンドではない。

`wda tools fetch`は引数を取らない。このWDAリリースが内蔵する1つのTypeScriptツールバージョンだけを、現在のプラットフォーム向けに復元する。バージョンを切り替えず、`anchor.json`や`package.json`も変更しない。バージョン、タグ、範囲、URL、その他の引数は使用法エラーであり、終了ステータスは`2`である。

WDAに`wda deps`コマンドはない。ウェブサイトのパッケージはプロジェクト自身のパッケージマネージャーが扱う。たとえば`npm install <package>`や`bun add <package>`を使う。`wda deps`は、サブコマンドの有無にかかわらず使用法エラーであり、終了ステータスは`2`で、外部のパッケージマネージャーを示すヒントを表示する。WDAは`sync`、`refresh`、`add`などの別名を残さない。

引数なしで`wda`を実行すると、各コマンドと`--version`の使い方をstdoutに出力し、stderrは空のまま、終了ステータス`0`で終了する。この出力は使い方の説明であり、別のコマンドでもコマンドライン使用法エラーでもない。

## 外部ツール

### gitとパッケージマネージャーの使い方

Gitが使える場合、`wda init`はローカルリポジトリと最初のコミットを作成する。GitがPATH上にない場合や起動できない場合も、初期化はWarningを出してプロジェクトを作成するが、ローカルリポジトリと最初のコミットは作成しない。`wda check`はGitを検査し、ツールチェーンの状態を報告する。

`wda check`と`wda build`は、パッケージマネージャーを起動せず、パッケージのインストールや更新を行わず、ネットワークも使わない。準備済みのプロジェクトでは、パッケージマネージャーがPATH上にある必要はない。両コマンドが読むのは、`node_modules`にインストール済みの内容とネイティブツールキャッシュだけである。外部のパッケージマネージャーを呼び出すコマンドは`wda init`だけであり、それも既定のパッケージをインストールするときに限る。使えるマネージャーがない場合や、マシンがregistryに到達できない場合、`wda init`は完全な`WDA Minimal`プロジェクトを作成する。

`wda check`、`wda build`、`wda tools fetch`は既存のGitリポジトリや`.gitignore`を必要としない。

### コマンドごとに必要なツール

| Command | `git` | Package manager | Native TypeScript tool cache |
| :--- | :--- | :--- | :--- |
| `init` | 任意。PATH上にないか起動できない場合でもプロジェクトは作成するが、`.git`と最初のコミットは作成せず、`wda.toolchain.git-missing` Warningを出す | 任意。使えるマネージャーがない場合、`init`は`WDA Minimal`を作成する | 不要。`init`は取得を試み、失敗してもWarningを出すだけである |
| `check` | 任意。PATH上にない場合は`wda.toolchain.git-missing` Warningを出力する。見つかったが起動できない場合は`wda.tool.fault` Warningを報告する。どちらの場合もコマンドは成功する | 起動しない。マネージャーがないだけでは問題にならない | ページが`scripts/**/*.ts`のエントリを参照する場合にだけ調べる。キャッシュがなければ`wda tools fetch`を示す`wda.build.tool-unavailable` Warningを出す |
| `build` | 不要 | 起動しない | ページがscriptエントリを参照する場合にだけ必要である。純粋なHTMLプロジェクトはキャッシュなしでビルドできる。キャッシュがない場合は`wda.build.tool-unavailable` Errorになる |
| `tools fetch` | 不要 | 不要 | キャッシュを復元する。`init`を除き、ツールのためにネットワークを使う唯一のコマンドである |

`wda check`は、`package.json`が宣言していても`node_modules`にないパッケージを見つけると、パッケージ名を示し、`packageManager`に記録されたマネージャーを指す`wda.build.dependency-unlocked` Warningを報告する。scriptエントリがあるのにツールキャッシュがない場合は、`wda.build.tool-unavailable` Warningを報告する。これらのWarningは成功ステータスを変えず、`check`は何も書き込まない。マネージャーのインストールコマンドや`wda tools fetch`は自分で実行する。

`wda init`でGitが見つからないか起動できない場合でも、プロジェクトと`.gitignore`は作成し、`.git`と最初のコミットは省略し、`wda.toolchain.git-missing` Warningを出す。Denoがないことは、どの場面でも条件にならない。WDAはDenoを使わなくなり、`wda.toolchain.deno-missing`を出すコマンドはない。

## wda init

### 作成するもの

`wda init [--package-manager <name>]`は現在のディレクトリに新しいプロジェクトを作成し、既定値をそのまま使用する。対応するマネージャー名は`npm`と`bun`である。未知の名前は使用法エラーになる。マネージャーを指定しない場合、`init`は対応するマネージャーだけを調べ、調べる間は何もダウンロードせず、使えるものが1つだけならそれを選ぶ。複数使える場合、対話端末ではメニューを表示し、非対話実行では`--package-manager`を求める使用法エラーで終了する。メニューを取り消すと、何も書き込まずにコマンドが終了する。

`WDA Minimal`経路では次のファイルを作成する。

```text
.git/                   Local Git repository (skipped when Git is unavailable)
.gitignore              Content is fixed to /dist/ and /node_modules/
README.md
anchor.json
package.json
docs/architecture.md
docs/design.md
docs/glossary.md
pages/index.html
tokens/tokens.json
```

`anchor.json`はWDAの設定と、実際に使うDesign Systemを保持する。`package.json`はウェブサイトのパッケージを宣言する唯一の場所である。`WDA Minimal`には依存関係も`packageManager`フィールドもなく、最初の外部インストールまでマネージャーのロックファイルもない。フォールバックが起きた場合、`anchor.json`には、initが本来使おうとしたDesign Systemとフォールバックの`reason`をテキストで記録する。

Spectrum 2経路では`scripts/main.ts`も作成する。マネージャーが1つに決まりregistryが応答する場合、`init`は最新の安定版Spectrumの正確なバージョンを解決し、専用のステージディレクトリでそのマネージャーを使ってSpectrum 2の6つのパッケージと`lit`をインストールし、マネージャー名と正確なバージョンを`package.json`の`packageManager`フィールドに記録する。ロックファイルと`node_modules`はマネージャーが管理する。

マシンがオフラインの場合、使えるマネージャーがない場合、または指定したマネージャーが使えない場合、`init`は完全な`WDA Minimal`プロジェクトを作成する。ローカルのパッケージキャッシュがあっても、`init`がSpectrumに切り替わることはない。`WDA Minimal`ではマネージャーを実行しない。

Gitが使える場合、プログラムは`git init`を実行し、ディレクトリ全体を初期コミットに記録する。メッセージは`wda init`に固定され、リモートリポジトリは設定しない。既存のDesignerファイルもコミットに含まれる。GitがPATH上にない場合や起動できない場合でも、`init`はプロジェクトと`.gitignore`を作成し、`.git`と最初のコミットを省略して、`wda.toolchain.git-missing` Warningを出す。

実行に成功した後、`init`は内蔵のTypeScriptツールキャッシュの復元も試みるが、この実行中にregistryが応答した場合に限る。失敗した場合は`wda tools fetch`を示す`wda.init.tool-unavailable` Warningを出し、プロジェクトは失敗にならない。

`.gitignore`は初期化時だけ作成する。`wda init`の完了後、WDAコマンドがこのファイルを読み書きすることはない。

### 衝突規則

対象ディレクトリには他のファイルがあってよい。`wda init`が作成する必要のあるファイルまたはディレクトリがすでに存在する場合、プログラムは書き込み前に中止する。検査対象は上記の項目、必要なディレクトリ、`.git/`、既存の`package.json`、旧形式の`wda.json`、`node_modules`、対応する各マネージャーのロックファイルと状態ファイルである。この範囲外のファイルは読み取り、変更、マージしない。`init`は既存のインストール結果を引き継がない。

### Design Systemの解決とフォールバック

`wda init`はまずregistryから最新の安定版Spectrumの正確なバージョンを解決する。解決に成功すると、Design Systemを`anchor.json`に書き込み、選んだマネージャーで依存パッケージをインストールする。解決またはインストールに失敗すると、この実行がSpectrum用に作成した内容を削除し、`WDA Minimal`にフォールバックし、使用したDesign Systemと理由をstdoutで説明する。理由はマネージャーが報告したものであり、後でSkillにSpectrumとLitを追加してもらえることも出力で示す。registryに到達できない場合の表現は「registry unreachable」であり、マシンがオフラインだと断定しない。Minimalへのフォールバックは実行失敗ではない。解決失敗とみなす条件は`docs/design-system.md`を参照する。

### 失敗時のロールバックとクリーンアップ

`wda init`は初期化全体を一度の処理として実行する。いずれかの書き込みに失敗した場合やメニューを取り消した場合、書き込んだものを削除し、実行前のディレクトリ状態に戻す。作成した順序の逆順でファイルを削除し、この実行が作成した内容だけを対象にする。Spectrum 2のパッケージのインストールに失敗してWDA Minimalへフォールバックする場合、まずSpectrum 2固有のファイルを削除し、その後Minimalプロジェクトを作成する。クリーンアップに失敗した場合は明確なエラーであり、成功とは主張しない。マネージャーが実行したサードパーティのパッケージスクリプトの影響は、元に戻せるとは約束しない。

### 拒否条件と診断コード

| Diagnostic code | Severity | Condition |
| :--- | :--- | :--- |
| `wda.init.tool-unavailable` | Warning | プロジェクトの書き込み後に、内蔵のTypeScriptツールパッケージを復元できなかった。プロジェクトは完全であり、`wda tools fetch`で再試行できる |
| `wda.init.path-conflict` | Error | 名前またはパスの衝突が発生した（衝突規則）。何も書き込まない |
| `wda.init.git-identity-fallback` | Warning | 環境にGitユーザーIDが設定されていないため、一時IDを使ってinitのコミットを完了する |

## ウェブサイトのパッケージ

WDAはウェブサイトのパッケージを解決、ダウンロード、ロック、更新しない。`package.json`がそれらを宣言し、プロジェクトが使うパッケージマネージャーがロックファイルと`node_modules`を管理する。パッケージの追加、削除、更新にはマネージャー自身のコマンドを使う。たとえば`npm install <package>`や`bun add <package>`を実行し、その後`wda check`または`wda build`を実行する。

`wda check`と`wda build`はパッケージをインストールも更新もしない。パッケージとネイティブツールキャッシュが揃っていれば、ネットワークなしで動作する。パッケージやツールキャッシュが欠けている場合は明確な診断を出し、コマンドが黙ってダウンロードすることはない。パッケージのインストールはサードパーティのコードを実行するため、自分で開始する別の手順である。

`wda deps`コマンドは削除された。実行すると`wda.cli.usage`、終了ステータス`2`、外部のパッケージマネージャーを示すヒントが返る。

## wda check

### buildと検証エンジンを共有

`wda check`は読み取り専用でプロジェクトを検査する。ファイルを書き込まず、`wdaVersion`も更新しない。`wda build`と同じプロジェクト契約、文書、HTML、Tokensの検証規則を使う。プロジェクトの正しさは、ユーザーが先に`check`を実行したかどうかに左右されてはならない。

`check`はさらにpreflightを実行し、このコンピューターでプロジェクトが必要とするものを調べる。

- GitがPATH上にない場合、プログラムは`wda.toolchain.git-missing` Warningを報告する。Gitが存在しても起動できない場合、`wda.tool.fault` Warningを報告する。
- `package.json`が宣言していても`node_modules`にない各パッケージについて、パッケージ名と`packageManager`に記録されたマネージャーを示す`wda.build.dependency-unlocked` Warningを1件報告する。
- ページが`scripts/**/*.ts`のエントリを参照しているのにネイティブTypeScriptツールキャッシュがない場合、`wda tools fetch`を示す`wda.build.tool-unavailable` Warningを報告する。scriptエントリのないプロジェクトにキャッシュは不要であり、このWarningも出ない。

このpreflightは、パッケージマネージャーを起動せず、インストールせず、書き込まない。PATH上のパッケージマネージャーも調べないため、マネージャーがないだけでインストール済みのプロジェクトが壊れて見えることはない。これらのWarningは成功ステータスを変えない。BuildはPATH上のツールを検査しない。また、パッケージやツールキャッシュがないと先へ進めないため、BuildはそれらをErrorとして報告する。

CheckとBuildはどちらも、次の4つのプロジェクト検証ルールモジュールを実行する。

- `src/validation/project_contract.rs`
- `src/validation/documents.rs`
- `src/validation/html.rs`
- `src/validation/tokens.rs`

Checkはさらに`src/validation/toolchain.rs`を実行し、ここに上記のpreflightがある。Buildはさらに`src/validation/build_output.rs`の出力規則を実行する。

### ルールモジュール：プロジェクト契約

このモジュールは`anchor.json`と`package.json`の存在と、有効なJSONであることを検証する。`anchor.json`は`schemas/anchor.schema.json`に適合し、未知フィールドがあってはならない。`package.json`はルートがオブジェクトであり、`dependencies`がある場合は値が文字列のオブジェクトで、`packageManager`がある場合は名前、`@`、正確なバージョンでなければならない。`wda.json`は読み込みも変換もしない。`anchor.json`または`package.json`のいずれかが欠けているプロジェクトは`wda.contract.missing`のErrorになる。フィールド仕様は`docs/generated-project.md`を参照する。

| Diagnostic code | Severity |
| :--- | :--- |
| `wda.contract.missing` | Error |
| `wda.contract.malformed-json` | Error |
| `wda.contract.unknown-field` | Error |
| `wda.contract.schema-violation` | Error |

### ルールモジュール：文書

プログラムは必須の4文書、`README.md`、`docs/architecture.md`、`docs/design.md`、`docs/glossary.md`の存在を検査する。`docs/`の3文書にはYAML front matterが必要で、`title`、`status`、`updated`を空にしてはならない。`updated`の形式は`YYYY-MM-DD`である。他の`docs/**/*.md`ファイルは存在する場合にのみ検査し、front matterに問題があればWarningを報告する。front matterに`sourceRefs`がある場合、各項目はプロジェクト内のファイルまたはセクションに対応しなければならない。

文書の整合性検査は構造とメタデータのみを見る。文書の内容をコード、`anchor.json`、tokens、依存関係の状態と比較しない。内容の差異は人がレビューする。

| Diagnostic code | Severity |
| :--- | :--- |
| `wda.docs.required-document-missing` | Error |
| `wda.docs.front-matter-missing` | Error |
| `wda.docs.front-matter-malformed` | 必須文書はError、任意文書はWarning |
| `wda.docs.front-matter-field-invalid` | 必須文書はError、任意文書はWarning |
| `wda.docs.source-ref-unresolvable` | Warning |
| `wda.docs.source-ref-escapes-root` | Warning |

### ルールモジュール：HTML

HTMLモジュールが検査する規則は3つのみである。`html[lang]`が空でないこと、すべての`img`に`alt`属性があること、同じ文書内で`id`が重複しないことである。`alt`検査が確認するのは属性の存在だけであり、テキストの品質は評価しない。

その他のアクセシビリティ（a11y）機能は人がレビューする。`alt`テキストが有意義か、画像上のコントラスト、フォーカスの可視性と順序、キーボード操作、ズームとリフロー、動きなどが対象である。すべての自動検査に合格しても、WCAGへの適合を意味しない。`wda check`はこのための「needs human review」Warningを出さない。

| Diagnostic code | Severity |
| :--- | :--- |
| `wda.html.lang-missing` | Error |
| `wda.html.img-alt-missing` | Error |
| `wda.html.duplicate-id` | Error |

### ルールモジュール：Tokens

TokensモジュールはJSONと[Design System](design-system.ja.md)に記載されたWDA CSS tokenの対応範囲を検査する。ビルド出力の前に値、型、alias、正規化後の名前を検査する。

| Diagnostic code | Severity |
| :--- | :--- |
| `wda.tokens.malformed-json` | Error |
| `wda.tokens.value-missing` | Error |
| `wda.tokens.type-unresolvable` | Error |
| `wda.tokens.alias-unresolvable` | Error |
| `wda.tokens.unsupported` | Error |

## wda build

### 必須検証とBuild Outputルールモジュール

`wda build`は最初に`check`と同じ4つのプロジェクト検証ルールモジュールを実行する。Errorが1つでもあればBuildは中止し、`dist/`を変更しない。Warningはビルドを妨げない。Errorがない場合に限り、Buildは出力段階に進み、`src/validation/build_output.rs`の出力規則を実行する。

### wda-includeの展開とテンプレート言語の不在

WDA独自のビルド時構文は次のものだけである。

```text
<wda-include src="..."></wda-include>
```

これはCustom Elementの命名規則に従い、ビルド時にプレーンなHTMLへ展開される。実行時のWeb Componentではなく、このタグを`dist/`に残してはならない。`src`属性が指せるのはプロジェクト内のファイルのみである。プロジェクトルートの外を指してはならず、循環参照も作ってはならない。

プロジェクトはテンプレート言語を提供せず、`<slot>`、`if`、`for`、変数、式、データバインディングをサポートしない。一般的なレイアウトと静的コンテンツには、ネイティブのセマンティックHTMLとCSSを優先する。再利用可能な動作、製品上の意味、またはカプセル化されたコンポーネントが必要な場合にのみコンポーネントを作成する。

### 型検査とモジュールビルド

`wda build`は、ページが参照する`scripts/**/*.ts`の各エントリを、このWDAリリースが内蔵する単一のTypeScriptツールバージョンで型検査する。型検査は必ずバンドルより前に実行し、プロジェクトの`tsconfig`では弱められないstrict設定を固定する。解析できる型エラーはプロジェクトのErrorであり、終了ステータスは`1`である。ツールが起動できない、クラッシュする、またはタイムアウトする場合は、終了ステータス`2`のToolFaultになる。どのページもscriptエントリを参照しない場合、ビルドはツールキャッシュを調べず、ツールも起動しない。

ツールは内蔵またはキャッシュ済みのプラットフォームパッケージから取得する。キャッシュがない場合、ビルドは`wda tools fetch`を示す`wda.build.tool-unavailable` Errorを報告し、`dist/`を変更しない。ビルドがツールをダウンロードすることはない。

その後、WDAはネイティブのRust製バンドラー（Rolldown）を、ブラウザーモード、ES module出力で使ってエントリをビルドする。バンドラーは、プロジェクト自身のパッケージマネージャーが`node_modules`にインストール済みのパッケージを解決する。WDAはパッケージのバージョンを解決せず、CDNミラーも持たない。リモートモジュールのインポート、Node専用モジュール、完全に出力されると証明できない動的インポートはErrorとして報告する。すべてのnpmパッケージがブラウザーで動作するとは主張しない。

### 一時ビルドワークスペース

`wda build`はバンドラーの出力をまず新しいステージングディレクトリに書き込み、一覧にあるエントリ、チャンク、静的アセットだけを`dist/`にコミットする。`node_modules`全体をコピーすることはない。`package.json`とマネージャーのロックファイルは、ビルド中に1バイトも変化しない。ビルド終了後、成功または失敗にかかわらずステージングディレクトリを削除する。

### インストール済みパッケージのみを使う

`wda build`は`node_modules`にインストール済みの内容だけを読む。パッケージをインストール、選択、アップグレードせず、ネットワークも使わない。scriptがインストールされていないパッケージをインポートすると、`wda.build.dependency-unlocked` Errorを報告し、自分のパッケージマネージャーでプロジェクトのパッケージをインストールするよう伝える。`dist/`の出力にリモート実行時依存関係は残らず、ウェブサイトの実行時にNode.js、Bun、WDAは不要である。

### dist/から秘密情報を除外

ビルドはdotfileや`.env*`ファイルをコピーしない。ページまたはスタイルがこれらのパスを参照すると、BuildはErrorを報告して中止する。この除外規則はファイルパスのみを調べ、ファイル内容は検査しない。

### 成功時にdist/の絶対パスを出力

ビルドに成功すると、プログラムは`anchor.json`の`wdaVersion`フィールドを更新し、`dist/`の絶対パスをstdoutに出力する。ビルドに失敗すると、`wdaVersion`を更新しない。ES modules、`fetch`、または`file://`で動作しない既知の機能を検出すると、出力にはHTTPホストが必要であることも知らせる。

### 拒否条件と診断コード

| Diagnostic code | Severity | Condition |
| :--- | :--- | :--- |
| `wda.build.tool-unavailable` | buildではError、checkではWarning | scriptエントリがネイティブTypeScriptツールキャッシュを必要とするが、キャッシュがないか場所を特定できない。ビルドは失敗し、`wda tools fetch`を示す。診断コードと重大度は安定した契約であり、要約文はその契約に含まれない |
| `wda.build.tokens-css-in-source` | Error | ソース側に`styles/tokens.css`が存在する |
| `wda.build.include-unresolvable` | Error | `wda-include`の`src`が存在しないファイルを指す |
| `wda.build.include-escapes-root` | Error | `wda-include`の`src`がプロジェクトルートの外を指す |
| `wda.build.include-cycle` | Error | `wda-include`が循環参照を作る |
| `wda.build.type-error` | Error | TypeScriptの型検査に失敗する |
| `wda.build.secret-path-referenced` | Error | ページまたはスタイルがdotfileか`.env*`を参照する |
| `wda.build.reference-unresolvable` | Error | ページまたはスタイルが参照するファイルが存在しない、またはscriptがリモートモジュールをインポートする |
| `wda.build.unreferenced-asset` | Warning | `assets/`ディレクトリにページまたはスタイルから参照されていないファイルがある |
| `wda.build.output-path-collision` | Error | 2つのソースパスが同じ`dist/`パスに対応する |
| `wda.build.dependency-unlocked` | buildではError、checkではWarning | `package.json`が宣言する、またはscriptがインポートするパッケージが`node_modules`にインストールされていない。この名前は歴史的なものであり、現在は「未インストール」を意味し、ロックファイルは参照しない |

## 診断出力

### 重大度は影響で決まる

すべての診断は重大度と診断コードを含む。プログラムは実際の影響に基づいて重大度を決める。たとえば、問題の分類を重大度へ直接割り当て、a11y問題をすべてWarning、スクリプト問題をすべてErrorにすることはない。問題が具体的で、ユーザーが改善のために行動できる場合にのみWarningを出す。ユーザーが解消できない固定の「needs human review」通知は表示しない。

| Severity | Meaning |
| :--- | :--- |
| Error | ビルド結果またはプロジェクトを不正にし、ビルドを止める必要がある問題。成果物が誤ると判定できる場合はErrorにできる |
| Warning | ビルドを止めないが、正しさまたは品質への懸念を示す問題。DesignerとAIが簡単に見落とさないよう、十分に目立ち、繰り返し表示されなければならない |

### 外部向けに安定した意味

診断出力には特定の構造化形式がない。外部の利用者に対して安定するのは、次の5項目のみである。

| Item | Content |
| :--- | :--- |
| severity | 重大度 |
| diagnostic code | 診断コード自体 |
| location | 正規化したプロジェクト相対パスと行番号、列番号（line:col） |
| output stream | stderrとstdoutは分離される |
| exit status | 終了ステータスコード |

メッセージと次の手順の提案は存在し、明確で分かりやすくなければならないが、正確な表現は後の変更で変わることがある。句読点、区切り文字、列幅は契約に含まれない。利用側はテキストをフィールドに分割して解析してはならない。

### 終了ステータス

`src/main.rs`は返された結果から終了ステータスを設定する。`init`、`check`、`build`が`ValidationReport`を通じて報告する診断にErrorが含まれる場合、ステータスコードは`1`である。`wda.cli.usage`、ツールエラー、その他のToolFaultはステータスコード`2`となる。`wda tools fetch`はツールキャッシュを復元できない場合、ステータスコード`1`で終了する。

`wda.init.path-conflict`と（Errorとしての）`wda.build.tool-unavailable`は`ValidationReport`経由で報告されるため、ステータスコードは`1`である。`wda.init.tool-unavailable`と`check`のpreflightのWarningが、ステータスコードを変えることはない。

```text
0   Success; Warnings may exist
1   The `ValidationReport` contains an Error
2   CLI usage error (including the removed `wda deps`), or ToolFault
```

### 出力ストリーム

診断はstderrへ出力する。成功結果とその他の要約メッセージはstdoutへ出力する。これには`wda init`のDesign Systemの説明と、`wda build`の`dist/`パスも含まれる。

### 1行で判定でき、人間とAIが読める

各診断は1行である。同じ入力と同じPATH状態では、同じバイナリが同じ診断一式を同じ順序で出力する。Checkのツールチェーン診断は、GitがPATH上にあり起動できるか、`node_modules`にどのパッケージがインストールされているか、ツールキャッシュがあるかによって変わる。順序には明示的な同順位判定があるため、ファイルシステムの走査順序に影響されない。利用者は診断コードと重大度を読むだけでよく、テキストを固定フィールドに分割する必要はない。各診断はメッセージと次の手順の提案を含み、表現はバージョン間で変わることがある。

### JSON出力

`wda check --json`と`wda build --json`は末尾に改行を付けたUTF-8 JSON文書を1件stdoutに出力する。サブプロセスの出力は捕捉し、stderrにレポートを再出力しない。OSによる終了やstdoutへの書き込み失敗時は文書の完成を保証できない。

文書は`schema_version` 1、`command`（`check`または`build`）、`status`（`success`、`project-error`、`usage-error`、`tool-fault`）、`diagnostics`、`summary`を持つ。各診断は`code`、`severity`（`error`または`warning`）、`path`、`line`、`column`、`message`、`next_action`を持つ。位置が不明な項目はnullで、パスの区切りはスラッシュである。メッセージの制御文字はJSON値として保持する。

`summary`は`errors`、`warnings`、`dist_path`、`needs_http`を持つ。成功したbuildだけがプロジェクト相対のパスと真偽値を返す。checkとすべての失敗では古いdistがあっても両方nullである。Warningのみなら成功し終了コードは0、プロジェクトエラーは1、使用法エラーとツール障害は2である。JSONと人向け出力の診断の順序は同じである。

JSONモードには先頭引数の`check`または`build`と、`--`終端子より前の正確な`--json`が必要である。このモードでは未知または重複した引数もJSONの使用法エラーとなる。ほかの呼び出しは人向けの使用法を維持する。

### パスの正規化と制御文字の保護

プログラムはプロジェクトルートからの相対パスをスラッシュ（`/`）区切りで出力し、ホストの絶対パスを漏らさない。プロジェクトファイルに改行や制御文字が含まれていても、それを使って偽の診断を挿入することはできない。

### コマンド間で共通する診断コード

| Diagnostic code | Severity | Condition |
| :--- | :--- | :--- |
| `wda.cli.usage` | Error | コマンドラインの使い方が誤っている。終了ステータスは`2` |
| `wda.cli.json-not-supported` | Error | checkまたはbuild以外で`--json`を渡した。終了ステータスは`2` |
| `wda.tool.fault` | Error or Warning | 外部ツールまたはファイルシステムが失敗した。重大度はコマンドと影響によって決まる。Errorの終了ステータスは`2` |
| `wda.command.not-implemented` | Error | 予約済みコード。この診断コードを出すコマンドは現在ない |

### 診断コード一覧

`src/codes.rs`だけが診断コードを宣言する。この文書は検索表であり、すべてのコードを1回ずつ掲載する。

| Diagnostic code | Severity | Command |
| :--- | :--- | :--- |
| `wda.command.not-implemented` | Error | Cross-command |
| `wda.cli.json-not-supported` | Error | Cross-command |
| `wda.cli.usage` | Error | Cross-command |
| `wda.tool.fault` | Error or Warning | Cross-command |
| `wda.toolchain.git-missing` | Warning | check / init |
| `wda.toolchain.deno-missing` | Retired | none. No command emits it; the declaration stays in the registry until a later cleanup removes it |
| `wda.contract.missing` | Error | check / build |
| `wda.contract.malformed-json` | Error | check / build |
| `wda.contract.unknown-field` | Error | check / build |
| `wda.contract.schema-violation` | Error | check / build |
| `wda.docs.required-document-missing` | Error | check / build |
| `wda.docs.front-matter-missing` | Error | check / build |
| `wda.docs.front-matter-malformed` | Error or Warning | check / build |
| `wda.docs.front-matter-field-invalid` | Error or Warning | check / build |
| `wda.docs.source-ref-unresolvable` | Warning | check / build |
| `wda.docs.source-ref-escapes-root` | Warning | check / build |
| `wda.html.lang-missing` | Error | check / build |
| `wda.html.img-alt-missing` | Error | check / build |
| `wda.html.duplicate-id` | Error | check / build |
| `wda.tokens.malformed-json` | Error | check / build |
| `wda.tokens.value-missing` | Error | check / build |
| `wda.tokens.type-unresolvable` | Error | check / build |
| `wda.tokens.alias-unresolvable` | Error | check / build |
| `wda.init.tool-unavailable` | Warning | init |
| `wda.init.path-conflict` | Error | init |
| `wda.init.git-identity-fallback` | Warning | init |
| `wda.build.tool-unavailable` | Error or Warning | check / build |
| `wda.build.tokens-css-in-source` | Error | build |
| `wda.build.include-unresolvable` | Error | build |
| `wda.build.include-escapes-root` | Error | build |
| `wda.build.include-cycle` | Error | build |
| `wda.build.type-error` | Error | build |
| `wda.build.secret-path-referenced` | Error | build |
| `wda.build.reference-unresolvable` | Error | build |
| `wda.build.unreferenced-asset` | Warning | build |
| `wda.build.output-path-collision` | Error | build |
| `wda.build.dependency-unlocked` | Error or Warning | check / build |
