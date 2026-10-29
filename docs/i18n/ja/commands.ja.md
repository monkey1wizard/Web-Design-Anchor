---
title: "コマンド"
status: stable
updated: 2026-10-15
type: Reference
description: "4つのコマンドのツール要件、動作、診断、終了ステータスを説明する。"
tags: []
source: docs/commands.md
lang: ja
source_sha256: 229687712163776076309291447a0984727e85b20ec0353f5e2edb85fe3980aa
translated_at: 2026-10-23
---

# コマンド

Coreは4つのコマンドを提供する。DesignerとAIはプロジェクトファイルを直接編集し、Coreは一般的な編集を行わない。Designerのワークフロー、Skillのインストール、プレビューによってコマンドが増えることはない。

```text
wda init    Create a new project and a local Git repository
wda deps    Manage the dependency graph
wda check   Validate without producing output
wda build   Validate, then build the output in dist/ if validation passes
```

`wda --version`はバージョン情報を表示するだけであり、Skillがバージョンを比較するために使う。5つ目のコマンドではない。

引数なしで`wda`を実行すると、4つのコマンドと`--version`の使い方をstdoutに出力し、stderrは空のまま、終了ステータス`0`で終了する。この出力は使い方の説明であり、5つ目のコマンドでもコマンドライン使用法エラーでもない。

## 外部ツール

### PATH上のdenoとgitの使い方

`deno`は依存パッケージを解決し、トランスパイラーを呼び出す。`wda init`はローカルリポジトリの作成に`git`を必要とし、ファイルを書き込む前にGitを起動できるか確認する。GitがPATH上にない場合、または起動できない場合、初期化は中止される。`wda check`はGitを検査し、ツールチェーンの状態を報告する。`deno`がなくてもMinimal初期化を実行できる。初期化で依存パッケージの解決またはインストールにDenoが必要で、DenoがPATH上にないか起動できない場合、プログラムは`WDA Minimal`にフォールバックする。

`wda check`、`wda build`、`wda deps`は既存のGitリポジトリや`.gitignore`を必要としない。

### esbuildのバージョンはWDAが固定し、deno経由で呼び出す

WDAは`deno run -A npm:esbuild@<pinned version>`でTypeScriptトランスパイラーを呼び出す。WDAがesbuildのバージョンを固定するため、PATH上に`esbuild`やNode.jsは必要ない。

### コマンドごとに必要なツール

| Command | `deno` | `git` |
| :--- | :--- | :--- |
| `init` | 任意。DenoがPATH上にないか起動できない場合、オンラインのDesign System解決や依存関係のインストールを行わず、WDA Minimalで制限付き初期化を行う | 必須。PATH上にないか起動できない場合、Errorで中止し、何も書き込まない |
| `deps` | 必須 | 不要 |
| `check` | 任意。PATH上にない場合は`wda.toolchain.deno-missing` Warningを出力する。見つかったが起動できない場合は`wda.tool.fault` Warningを報告する。どちらの場合もコマンドは成功する | 任意。PATH上にない場合は`wda.toolchain.git-missing` Warningを出力する。見つかったが起動できない場合は`wda.tool.fault` Warningを報告する。どちらの場合もコマンドは成功する |
| `build` | 必須 | 不要 |

GitまたはDenoがPATH上にない場合、または見つかっても起動できない場合、`wda check`が出すのはWarningのみである。それだけでコマンドが失敗することはない。
`wda init`でGitが見つからないか起動できない場合、`wda.init.tool-unavailable` Errorを出して書き込み前に中止する。DenoがPATH上にないか起動できない場合、`wda init`はWDA Minimalで制限付き初期化を行う。この経路ではオンラインのDesign System解決や依存関係のインストールを行わない。DenoがPATH上にない場合は`wda.toolchain.deno-missing` Warningも出す。

## wda init

### 作成するもの

`wda init`は現在のディレクトリに新しいプロジェクトを作成し、既定値をそのまま使用する。

`WDA Minimal`経路では次のファイルを作成する。

```text
.git/                   Local Git repository
.gitignore              Content is fixed to /dist/
README.md
wda.json
docs/architecture.md
docs/design.md
docs/glossary.md
pages/index.html
tokens/tokens.json
```

ネットワークが利用できる場合、`WDA Minimal`は`lit`のインストールを試みる。インストールに成功すると、`deno.jsonc`と`deno.lock`を生成する。オフラインの場合、またはインストールに失敗した場合でも初期化は完了するが、依存関係ファイルは生成しない。stdoutで理由を説明し、`wda deps add lit`の実行を提案する。`WDA Minimal`は既定でstarter scriptを含まない。

Spectrum 2経路では`scripts/main.ts`も作成する。オンライン解決に成功すると、Spectrum 2の6つのパッケージと`lit`をインストールし、`deno.jsonc`と`deno.lock`を生成する。starter scriptが実際にインポートするパッケージは、同じバージョンでロックする。

ファイルを書き込んだ後、プログラムは`git init`を実行し、ディレクトリ全体を初期コミットに記録する。メッセージは`wda init`に固定され、リモートリポジトリは設定しない。既存のDesignerファイルもコミットに含まれる。

`.gitignore`は初期化時だけ作成する。`wda init`の完了後、WDAコマンドがこのファイルを読み書きすることはない。

### 衝突規則

対象ディレクトリには他のファイルがあってよい。`wda init`が作成する必要のあるファイルまたはディレクトリがすでに存在する場合、プログラムは書き込み前に中止する。検査対象は上記の項目、必要なディレクトリ、`.git/`、`deno.jsonc`、プロジェクト側の`deno.json`、`deno.lock`である。この範囲外のファイルは読み取り、変更、マージしない。

### Design Systemの解決とフォールバック

`wda init`はまずオンラインで最新の安定版Spectrumの正確なバージョンを解決する。解決に成功すると、そのバージョンを`wda.json`に書き込み、依存パッケージをインストールする。解決またはインストールに失敗すると、`WDA Minimal`にフォールバックし、使用したDesign Systemと理由をstdoutで説明する。インストール失敗時には復旧手順も示す。Minimalへのフォールバックは実行失敗ではない。解決失敗とみなす条件は`docs/design-system.md`を参照する。

### 失敗時のロールバックとクリーンアップ

`wda init`は初期化全体を一度の処理として実行する。いずれかの書き込みに失敗すると、書き込んだものを削除し、実行前のディレクトリ状態に戻す。クリーンアップ対象にはSpectrum 2とWDA Minimalが作成する`deno.jsonc`と`deno.lock`も含まれる。作成した順序の逆順でファイルを削除する。Spectrum 2の依存パッケージのインストールに失敗してWDA Minimalへフォールバックする場合、まずSpectrum 2固有のファイルを削除し、その後Minimalプロジェクトを作成する。

### 拒否条件と診断コード

| Diagnostic code | Severity | Condition |
| :--- | :--- | :--- |
| `wda.init.tool-unavailable` | Error | `git`がPATH上にないか、見つかっても起動できない。メッセージは実際の理由を示す。何も書き込まない |
| `wda.init.path-conflict` | Error | 名前またはパスの衝突が発生した（衝突規則）。何も書き込まない |
| `wda.init.git-identity-fallback` | Warning | 環境にGitユーザーIDが設定されていないため、一時IDを使ってinitのコミットを完了する |

## wda deps

### 宣言とロック

現行プロジェクトは`deno.jsonc`の`imports`フィールドでnpmパッケージを宣言し、固定バージョンの`npm:<package>@<version>`指定子を使い、`deno.lock`にバージョンをロックする。WDAは既存の`package.json`プロジェクトもサポートする。`wda deps`は最初にプロジェクト形式を判定する。ルートに`package.json`があれば旧形式の経路を使い、なければ現行の`deno.jsonc`経路を使う。

`wda deps`は`deno`を呼び出して`deno.lock`を解決、更新する。現行形式では`--config deno.jsonc`、旧形式では`--package-json`を使う。Denoがファイルを書き込み、`deno.jsonc`のコメント、末尾カンマ、その他の独自フィールドを保持する。依存関係グラフを変更するのはこのコマンドのみである。`wda build`はロック済みバージョンだけを使い、独自に新しいバージョンを選ぶことはない。

### add、remove、updateの役割

```text
wda deps add <package>      Add one dependency package and lock it
wda deps remove <package>   Remove one dependency package and update the lock
wda deps update [package]   Re-resolve and lock within the existing declaration
```

### 段階的検証とアトミックなコミット

`wda deps`は最初に分離した一時ディレクトリでDenoコマンドを実行し、`deno install --frozen`で依存関係グラフを検証する（段階的検証）。検証に成功した後でのみ、プロジェクトファイルを変更する。実行時に`--allow-scripts`を渡さないため、npmのライフサイクルスクリプトは実行しない。

検証後、WDAは変更を決められた順序で一度に反映する。まず`deno.lock`を書き込み、名前を変更するか削除し、その後に依存関係宣言ファイル（`deno.jsonc`または`package.json`）を更新する。この処理中にI/Oエラーが起きると、プログラムはファイルの元のバイト列を復元し、破損または不整合な状態を残さない。

### 意味上の空状態規則

実際の依存パッケージがないプロジェクトでは、空の宣言ファイルやロックファイルを保持しない。

- 現行の`deno.jsonc`プロジェクトで最後の依存パッケージを削除すると、WDAはルートレベルの意味構造を検査する。空の`imports`以外に意味のあるフィールドがなければ、コメントだけではファイルを保持する理由にならないため、`deno.jsonc`を削除する。他の独自ルートフィールドがあれば、ファイルを保持して`imports`だけを空にする。`deno.lock`にバージョンヘッダー（`{"version": ...}`）しかなければ、空の`deno.lock`も削除する。
- 旧形式の`package.json`プロジェクトでは、最後の依存パッケージを削除すると`package.json`と`deno.lock`の両方を削除する。
- オフラインで作成された`WDA Minimal`プロジェクトには、既定でこれら2ファイルは存在しない。

### 拒否条件、復旧、診断コード

`wda deps`はコマンド間で共通の診断コードを使ってErrorを報告する。

- プロジェクト側に`deno.json`がある場合、または`deno.json`と`deno.jsonc`の両方がある場合、`wda deps`は`wda.tool.fault`で拒否し、中止する。設定ファイルの名前を変更するか、`deno.jsonc`に切り替えるよう求める。WDAはプロジェクト側の`deno.json`を作成、編集するよう求めない。
- コマンドの使い方が誤っている場合、`wda.cli.usage`で拒否する（終了ステータス`2`）。
- 環境に`deno`がない、解決に失敗する、または段階的検証に失敗する場合、`wda.tool.fault`で拒否する（終了ステータス`2`）。実行に失敗した場合、何も書き込まず、プロジェクトファイルを変更しない。
- 復旧方法：初期化時にオフラインだったためLitがインストールされなかった場合、ネットワーク接続が戻った後に`wda deps add lit`を実行できる。

## wda check

### buildと検証エンジンを共有

`wda check`は読み取り専用でプロジェクトを検査する。ファイルを書き込まず、`wdaVersion`も更新しない。`wda build`と同じプロジェクト契約、文書、HTML、Tokensの検証規則を使う。`check`はPATH上の任意ツールも検査する。GitまたはDenoがPATH上にない場合、対応する`wda.toolchain.*-missing` Warningを報告する。ツールが存在しても起動できない場合、`wda.tool.fault` Warningを報告する。プロジェクトの正しさは、ユーザーが先に`check`を実行したかどうかに左右されてはならない。
これらのWarningは成功ステータスを変えない。BuildはPATH上のツールを検査しない。

CheckとBuildはどちらも、次の4つのプロジェクト検証ルールモジュールを実行する。

- `src/validation/project_contract.rs`
- `src/validation/documents.rs`
- `src/validation/html.rs`
- `src/validation/tokens.rs`

Checkはさらに`src/validation/toolchain.rs`を実行し、PATHの状態に応じたツールチェーン診断を報告する。BuildはPATH上の任意ツールを検査しない。BuildにDenoが必要でも利用できない場合、`wda.build.tool-unavailable` Errorを報告する。この診断の要約文はバージョン間で変わることがある。Buildはさらに`src/validation/build_output.rs`の出力規則を実行する。

### ルールモジュール：プロジェクト契約

このモジュールは`wda.json`の存在、有効なJSONであること、`schemas/wda.schema.json`への適合、未知フィールドがないことを検証する。フィールド仕様は`docs/generated-project.md`の`wda.json`フィールド表を参照する。

| Diagnostic code | Severity |
| :--- | :--- |
| `wda.contract.missing` | Error |
| `wda.contract.malformed-json` | Error |
| `wda.contract.unknown-field` | Error |
| `wda.contract.schema-violation` | Error |

### ルールモジュール：文書

プログラムは必須の4文書、`README.md`、`docs/architecture.md`、`docs/design.md`、`docs/glossary.md`の存在を検査する。`docs/`の3文書にはYAML front matterが必要で、`title`、`status`、`updated`を空にしてはならない。`updated`の形式は`YYYY-MM-DD`である。他の`docs/**/*.md`ファイルは存在する場合にのみ検査し、front matterに問題があればWarningを報告する。front matterに`sourceRefs`がある場合、各項目はプロジェクト内のファイルまたはセクションに対応しなければならない。

文書の整合性検査は構造とメタデータのみを見る。文書の内容をコード、`wda.json`、tokens、依存関係の状態と比較しない。内容の差異は人がレビューする。

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

Tokensモジュールは`tokens/tokens.json`が有効なJSONでDTCGに適合することを検査する。各tokenに値があり、型を判定でき、aliasを解決できることも検査する。

| Diagnostic code | Severity |
| :--- | :--- |
| `wda.tokens.malformed-json` | Error |
| `wda.tokens.value-missing` | Error |
| `wda.tokens.type-unresolvable` | Error |
| `wda.tokens.alias-unresolvable` | Error |

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

### 一時ビルドワークスペース

`wda build`はプロジェクトディレクトリの外に一時ワークスペース（build workspace）を作成する。WDAは専用JSON形式の`deno.json`をワークスペースに書き込む。これには、たとえば`nodeModulesDir: "manual"`や標準の`lib`一覧など、コンパイルとモジュールの設定が含まれる。

現行形式のプロジェクトでは、プロジェクト側の`deno.jsonc`にある`imports`をワークスペースの`deno.json`へマージし、他の独自フィールドは取り込まない。プロジェクト側に`deno.json`がある場合、BuildはToolFaultを報告して中止する。既存の`package.json`プロジェクトでは、そのファイルを変更せずにワークスペースへコピーする。

選択した形式の依存関係宣言が空でない場合に限り、プログラムはワークスペース内で`deno install --frozen`を実行する。現行形式では`imports`に1項目以上、旧形式では`dependencies`に1項目以上あることを意味する。宣言が空ならインストールを省略し、`deno.lock`も必要としない。実行時に`--allow-scripts`を渡さないため、npmのライフサイクルスクリプトは実行しない。ビルド終了後、成功または失敗にかかわらず一時ワークスペースを削除する。プロジェクトルートの`deno.lock`はビルド中に1バイトも変化しない。

### ロック済み依存関係のみを使う

`wda build`は`deno.lock`にロックされたバージョンのみを使う。新しいバージョンを選んだり、依存関係グラフを変更したり、暗黙にアップグレードしたりしない。ビルド時にすべての依存モジュールを`dist/`へバンドルし、リモート実行時依存関係を残さない。

### dist/から秘密情報を除外

ビルドはdotfileや`.env*`ファイルをコピーしない。ページまたはスタイルがこれらのパスを参照すると、BuildはErrorを報告して中止する。この除外規則はファイルパスのみを調べ、ファイル内容は検査しない。

### 成功時にdist/の絶対パスを出力

ビルドに成功すると、プログラムは`wda.json`の`wdaVersion`フィールドを更新し、`dist/`の絶対パスをstdoutに出力する。ビルドに失敗すると、`wdaVersion`を更新しない。ES modules、`fetch`、または`file://`で動作しない既知の機能を検出すると、出力にはHTTPホストが必要であることも知らせる。

### 拒否条件と診断コード

| Diagnostic code | Severity | Condition |
| :--- | :--- | :--- |
| `wda.build.tool-unavailable` | Error | ビルドにはDenoが必要だが利用できない。ビルドは失敗する。診断コードと重大度は安定した契約であり、要約文はその契約に含まれない |
| `wda.build.tokens-css-in-source` | Error | ソース側に`styles/tokens.css`が存在する |
| `wda.build.include-unresolvable` | Error | `wda-include`の`src`が存在しないファイルを指す |
| `wda.build.include-escapes-root` | Error | `wda-include`の`src`がプロジェクトルートの外を指す |
| `wda.build.include-cycle` | Error | `wda-include`が循環参照を作る |
| `wda.build.type-error` | Error | TypeScriptの型検査に失敗する |
| `wda.build.secret-path-referenced` | Error | ページまたはスタイルがdotfileか`.env*`を参照する |
| `wda.build.reference-unresolvable` | Error | ページまたはスタイルが参照するファイルが存在しない |
| `wda.build.unreferenced-asset` | Warning | `assets/`ディレクトリにページまたはスタイルから参照されていないファイルがある |
| `wda.build.output-path-collision` | Error | 2つのソースパスが同じ`dist/`パスに対応する |
| `wda.build.dependency-unlocked` | Error | 依存関係宣言（deno.jsoncまたはpackage.json）かソースコードが依存関係を参照しているが、deno.lockに対応するロック項目がない。またはロックファイルが古い |

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

`src/main.rs`は返された結果から終了ステータスを設定する。`init`、`check`、`build`が`ValidationReport`を通じて報告する診断にErrorが含まれる場合、ステータスコードは`1`である。`wda.cli.usage`、`deps`レベルの診断またはツールエラー、その他のToolFaultはステータスコード`2`となる。

`wda.init.tool-unavailable`と`wda.init.path-conflict`は`ValidationReport`経由で報告されるため、ステータスコードは`1`である。`wda.build.tool-unavailable`はToolFaultとして返されるため、ステータスコードは`2`である。

```text
0   Success; Warnings may exist
1   The `ValidationReport` contains an Error
2   CLI usage error, `deps` error, or ToolFault
```

### 出力ストリーム

診断はstderrへ出力する。成功結果とその他の要約メッセージはstdoutへ出力する。これには`wda init`のDesign Systemの説明と、`wda build`の`dist/`パスも含まれる。

### 1行で判定でき、人間とAIが読める

各診断は1行である。同じ入力と同じPATH状態では、同じバイナリが同じ診断一式を同じ順序で出力する。Checkのツールチェーン診断はGitとDenoがPATH上にあり、起動できるかによって変わる。順序には明示的な同順位判定があるため、ファイルシステムの走査順序に影響されない。利用者は診断コードと重大度を読むだけでよく、テキストを固定フィールドに分割する必要はない。各診断はメッセージと次の手順の提案を含み、表現はバージョン間で変わることがある。

### --jsonオプションはない

現在`--json`オプションや構造化診断形式はない。ユーザーが`--json`を渡すと、プログラムはstderrで拒否し、stdoutにJSONを出力せず、診断コード`wda.cli.json-not-supported`と終了ステータス`2`で終了する。

バージョン付き構造化出力を提案する場合、機械利用者を特定し、プレーンテキストのインターフェースに必要な情報がないことを示す再現可能な失敗例を添付しなければならない。この条件を満たした提案は、独立したADRとしてレビューに提出できる。「利用者がプレーンテキストを扱いたくない」という説明だけでは提案を裏付けるには不十分である。最初の段階で対象とするのは`check`と`build`のみである。

### パスの正規化と制御文字の保護

プログラムはプロジェクトルートからの相対パスをスラッシュ（`/`）区切りで出力し、ホストの絶対パスを漏らさない。プロジェクトファイルに改行や制御文字が含まれていても、それを使って偽の診断を挿入することはできない。

### コマンド間で共通する診断コード

| Diagnostic code | Severity | Condition |
| :--- | :--- | :--- |
| `wda.cli.usage` | Error | コマンドラインの使い方が誤っている。終了ステータスは`2` |
| `wda.cli.json-not-supported` | Error | ユーザーが`--json`オプションを渡した。終了ステータスは`2` |
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
| `wda.toolchain.git-missing` | Warning | check |
| `wda.toolchain.deno-missing` | Warning | check / init |
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
| `wda.init.tool-unavailable` | Error | init |
| `wda.init.path-conflict` | Error | init |
| `wda.init.git-identity-fallback` | Warning | init |
| `wda.build.tool-unavailable` | Error | build |
| `wda.build.tokens-css-in-source` | Error | build |
| `wda.build.include-unresolvable` | Error | build |
| `wda.build.include-escapes-root` | Error | build |
| `wda.build.include-cycle` | Error | build |
| `wda.build.type-error` | Error | build |
| `wda.build.secret-path-referenced` | Error | build |
| `wda.build.reference-unresolvable` | Error | build |
| `wda.build.unreferenced-asset` | Warning | build |
| `wda.build.output-path-collision` | Error | build |
| `wda.build.dependency-unlocked` | Error | build |
