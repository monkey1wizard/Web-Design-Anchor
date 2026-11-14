---
title: "Web Design Anchor"
status: stable
updated: 2026-11-12
type: Guide
description: "WDAの概要、クイックスタート、インストールガイド、主要コマンド、ドキュメント案内。"
tags: []
source: README.md
lang: ja
source_sha256: dcd326082d21c6402af6f275b925d8d23bdf9f28627af4a254e82b50b46425b2
translated_at: 2026-11-12
---

# Web Design Anchor

[English](../../../README.md) | [繁體中文](../zh-TW/README.zh-TW.md) | 日本語

**AIでWebサイトを作り、デザインの決定はプロジェクトに残す。**

![WDAのデモアニメーション：Designerが一文でAIにWebサイトの調整を依頼し、docs/design.md、wda check、wda buildが表示される](../../../assets/demo.gif)

これはデモアニメーションである。動画内の会話は一例である。

WDAを使うと、AIコーディングツールと一緒にシンプルな静的Webサイトを構築し、保守できる。コードを自分で書く必要はない。ただし、Claude CodeやCodexのように自分のコンピューターでコマンドを実行できるAIコーディングツールが必要である。コンピューターでコマンドを実行できないチャット画面だけでは足りない。Gitは任意であり、Gitがあるプロジェクトにはローカルの履歴が残る。生成されるWebプロジェクトは構成が明確で、ページ、スタイル、デザイン規則にそれぞれ決まった場所がある。色や文字サイズなどのデザインの選択は一か所にまとめてあり、サイト全体に対して変更できる。

WDAは標準化されたCLIとプロジェクト規則を提供する。デザイン規則はプロジェクト内の`docs/design.md`とDesign Tokensに保存するため、1回のAIとの会話に依存しない。AIはサイトを変更する前にこれらのファイルを読める。WDAは、AIがすべての規則に従うことを保証しない。結果は`wda check`で確認し、ページも自分で確認する。

技術面では、WDAはフロントエンドフレームワークに依存せず、標準Webに立ち戻る。HTML5とModern CSSのネイティブ機能で、豊かなレイアウトや動的な効果を実現できる。スクリプトにはTypeScriptを使い、保守しやすく整理されたコードを保つ。

ビルドと配布では、固定された1つのTypeScriptツールで型を検査し、ネイティブなRust製バンドラーでサイトをまとめる。すべての出力は`dist/`ディレクトリに書き込む。WDAのプログラム自体には、Deno、Node.js、Bunは不要である。利用者ごとにCoding AgentやIDEが異なるため、WDAにはプレビューサーバーを含めない。プレビューは各自の環境で行う。`dist/`にはWDAなしで動作する静的Webサイトが格納され、成果を直接確認し、いつでもデプロイできる。出力には、Litなどプロジェクトが使うサードパーティーのパッケージが含まれる場合がある。

## クイックスタート

4つの手順で最初のWebサイトを作成できる。

### 1. WDAと任意のツールをインストールする

WDA MinimalにDeno、Node.js、Bunは不要である。Gitは任意である。GitがPATHにあれば、WDAはプロジェクト用のローカルGitリポジトリも作成する。既定のSpectrumとLitのパッケージは、既存のパッケージマネージャーであるnpmまたはBunを使う。WDAがNode.jsやBunを自動でインストールすることはない。

WDAをインストールしたら、バージョンを確認する。

```sh
wda --version
```

詳しいインストール方法は、下記のインストールガイドを参照する。

### 2. 新しいプロジェクトを作成する

プロジェクト用の空のフォルダーを作成し、そのフォルダーに移動してから初期化コマンドを実行する。

```sh
mkdir my-site
cd my-site
wda init
```

`wda init`は現在のフォルダーに標準的なWDAプロジェクトを作成する。WDAの設定ファイル`anchor.json`、Webサイトのパッケージを宣言するファイル`package.json`、デザイン規則`docs/design.md`、ホームページ`pages/index.html`、Design Tokens`tokens/tokens.json`が含まれる。Gitが利用できる場合は、初回コミットを含むローカルGitリポジトリも作成する。フォルダーに`.git/`、`package.json`、または`wda init`が作成するファイルやフォルダーがすでにある場合、コマンドは何も書き込まずに停止する。利用できるパッケージマネージャーが複数ある場合は、`wda init --package-manager npm`または`wda init --package-manager bun`で使う名前を指定する。利用できるパッケージマネージャーがない場合やregistryに接続できない場合、`wda init`は完全なWDA Minimalプロジェクトを作成する。作成されるファイルの一覧は`docs/commands.md`を参照する。

### 3. AIとデザインする

変更を始める前に、プロジェクトの`docs/design.md`を読むようAIに依頼する。AIはこのファイルのDesign Systemの規則と、`tokens/tokens.json`のTokenを使って、HTML、CSS、Web Componentsを書ける。このファイルはAIがすべての規則に従うことを保証しないため、結果を`wda check`で確認し、ページも自分で確認する。

変更中はいつでも次の検証を実行できる。

```sh
wda check
```

`wda check`は、プロジェクト契約、必須ドキュメント、特定のHTML規則、Tokenの構造を検証する。見た目の品質やアクセシビリティの全項目は判定しない。それらはブラウザーでページを見て確認する。

### 4. デプロイ可能なWebサイトをビルドする

デザインが完成したら、ビルドコマンドを実行する。

```sh
wda build
```

WDAは、ページが参照するTypeScriptの型を検査し、すべての静的資産を`dist/`にまとめる。ビルドはパッケージマネージャーがすでにインストールしたパッケージを読む。パッケージのインストールは行わない。ページがTypeScriptスクリプトを使い、ツールのキャッシュがない場合は、`wda tools fetch`を実行してからもう一度ビルドする。ローカルのHTTP静的サーバーで結果を確認するか、`dist/`を静的サイトのホストにデプロイする。出力がES Moduleなど`file://`では動かない機能を使う場合、HTMLファイルを直接開いても正しく動作しない。

## インストールガイド

WDAは1つのプログラムであり、各プロジェクトは独自のWebサイトのパッケージを持つ。Gitとパッケージマネージャーは任意のツールである。Litは、既定のSpectrumが`package.json`で宣言するWebサイトのパッケージである。グローバルツールではなく、パッケージマネージャーがプロジェクト内にインストールする。

### Gitをインストールする（任意）

Gitを使うと、プロジェクトにローカルの履歴が残る。お使いのオペレーティングシステムに合う、優先する方法を1つ選ぶ。

| プラットフォーム | Git の優先パス | 公式の代替ソース |
| --- | --- | --- |
| Windows | `winget install --id Git.Git -e --source winget` | [Git for Windows インストーラー](https://git-scm.com/install/windows) |
| macOS | `brew install git` | [Git macOS インストールオプション](https://git-scm.com/download/mac) |
| Debian／Ubuntu | `sudo apt-get install git` | [Git Linux インストールオプション](https://git-scm.com/download/linux) |
| Fedora | `sudo dnf install git` | [Git Linux インストールオプション](https://git-scm.com/download/linux) |

インストール後、`git --version`を実行して動作を確認する。コマンドがGitを見つけられない場合は、新しいターミナルウィンドウを開いてから再確認する。Gitがない場合でも`wda init`はプロジェクトを作成し、Gitリポジトリと初回コミットを省いて警告を報告する。

### npmまたはBunをインストールする（任意、既定のパッケージ用）

既定のSpectrumとLitのパッケージには、PATHにnpmまたはBunが必要である。どちらか一方を公式のインストーラーでインストールする。WDAはどちらもインストールしない。WDA Minimalにもどちらも不要である。どちらもない場合、`wda init`はWDA Minimalを作成し、その後Skillが利用者の同意を得て既定のパッケージを追加できる。

### WDAをインストールする

公開GitHubリポジトリでは、安定版とprereleaseを提供している。利用可能なバージョンは[GitHub Releases](https://github.com/monkey1wizard/Web-Design-Anchor/releases)で確認する。

オペレーティングシステムに合わせて、インストール方法を1つ選ぶ。

#### macOSとLinux

```sh
brew install monkey1wizard/tap/wda
```

またはインストールスクリプトを使う。

```sh
curl -fsSL https://raw.githubusercontent.com/monkey1wizard/web-design-anchor/main/packaging/install.sh | bash
```

#### Windows

PowerShellのインストールスクリプトを使う。

```sh
irm https://raw.githubusercontent.com/monkey1wizard/web-design-anchor/main/packaging/install.ps1 | iex
```

WinGetパッケージはまだ審査中であり、現時点では利用できない。

#### Rustがインストール済みの場合

Rustツールチェーンがすでにある場合は、Cargoでインストールできる。

```sh
cargo install --git https://github.com/monkey1wizard/Web-Design-Anchor --locked
```

### TypeScriptツール

WDAは、このリリースに組み込まれた単一の公式TypeScriptバージョンでスクリプトを検査する。リリースビルドには、利用するプラットフォーム向けのパッケージが含まれる。ソースからのビルドでは、`wda init`の実行中にそれを復元しようとする。復元に失敗しても、`wda init`はプロジェクトを作成して警告を報告する。その後、引数なしで`wda tools fetch`を実行できる。このコマンドが復元するのは、このWDAリリースに結び付けられたバージョンだけである。`anchor.json`と`package.json`は変更しない。

### インストールを確認する

インストール後、`wda --version`を実行してWDAが起動することを確認する。Gitをインストールした場合は、`git --version`も実行する。例えば次のとおりである。

```sh
wda --version
```

ツールがPATHに見つからない場合は、インストール場所を確認してからターミナルを開き直す。ツールは見つかったが起動できない場合は、起動エラーを残したまま、実際のエラーに基づいて調べる。AIは終了ステータスを作り出してはならず、起動の失敗をツールが未インストールである証拠として扱ってはならない。

## 主要コマンド

WDAは、プロジェクトに関する3つのコマンドと、ツールに関する1つのコマンドを提供する。

| コマンド | 役割 | 説明 |
| :--- | :--- | :--- |
| `wda init` | プロジェクトの初期化 | 標準的なプロジェクト構造、`anchor.json`、`package.json`、デザインの基準文書、ホームページのテンプレートを作成する。既定のパッケージをインストールするときだけパッケージマネージャーを呼び出す |
| `wda check` | 静的検証 | 契約、ドキュメント構造、HTMLタグの規則、Tokenの参照を検証する。パッケージをインストールせず、ネットワークも使わない |
| `wda build` | ビルドと配布 | TypeScriptの型を検査し、インストール済みのパッケージをまとめ、すべての出力を`dist/`ディレクトリに書き込む。パッケージをインストールせず、ネットワークも使わない |
| `wda tools fetch` | TypeScriptツールのキャッシュ | このWDAリリースに組み込まれたTypeScriptツールを、現在のプラットフォーム向けに復元する。引数は受け付けない |

詳しいパラメーターと診断の出力については、`docs/commands.md`を参照する。

### Webサイトのパッケージ

Webサイトのパッケージのインストール、更新、ロックはパッケージマネージャーが管理する。例外として、`wda init`は既定のパッケージをインストールするためにマネージャーを呼び出す。パッケージの宣言は`package.json`が行い、ロックファイルと`node_modules`はお使いのパッケージマネージャーが管理する。パッケージの追加、削除、更新は、そのマネージャー自身のコマンドで行う。例えば`npm install <package>`や`bun add <package>`である。その後、`wda check`と`wda build`を実行する。

`wda deps`コマンドは削除された。実行すると、外部のパッケージマネージャーを案内する利用方法のエラーが表示される。WDAはこのコマンドのエイリアスを残さない。

## AIと協働する

WDAはAIとの協働を前提に設計されている。プロジェクトは公式のSkillを提供する。

- **公式Skill**：`skills/wda/`にある。CLIと同じリリースまたはソースリビジョンのSkillを使う。使用するCoding AgentがSkillに対応している場合は、このディレクトリを読み込む。SkillはAIにWDAのコマンドとプロジェクト規則の手順を与える。エージェントはSkillを自動では読み込まないことがあるため、AIがSkillを使っているかを確認する。使っていない場合は、AIにSkillを読み込ませるか、Skillがない環境の手順に従う。
- **Skillがない環境**：AIにWDA Skillがない場合は、まずこのREADMEまたは別の読み取り可能な公式WDAソースを渡す。AIがソースを読めない場合は、アクセス可能なソースを提供するよう求め、その後停止しなければならない。AIはインストールコマンドを推測してはならない。プロジェクトを作成した後は、そのプロジェクトの`docs/design.md`に基づいてコードを生成させ、各段階で`wda check`を実行して検証させる。

## ドキュメントの案内

| 知りたいこと | 参照するドキュメント |
| :--- | :--- |
| WDAの構成、境界、開発・検証・文書の規則 | `docs/architecture.md` |
| `wda init`が作成するプロジェクト：`anchor.json`と`package.json`のフィールド、プロジェクトツリー、`dist/`の対応 | `docs/generated-project.md` |
| 各コマンドの働き、診断出力の読み方 | `docs/commands.md` |
| Design Systemの既定とフォールバック、Token、Adapterの境界 | `docs/design-system.md` |
| Skillに書けること、その検証方法 | `docs/skill-spec.md` |
| 用語と語彙の定義 | `docs/glossary.md` |
| 決定済みの事項とその理由 | `docs/adr/` |
| WDA MinimalとSpectrumの具体的な定義 | `docs/design-systems/` |
| `wda init`がプロジェクトに書き込む文書テンプレート | `docs/templates/generated-project/` |
| セキュリティモデルの範囲と脆弱性の報告方法 | `SECURITY.md` |
| AIが`wda`を操作する方法 | `skills/wda/SKILL.md` |

## ライセンス

Apache-2.0である。サードパーティーの素材とコンポーネントのライセンスは`NOTICE`を参照する。
`docs/design.md`のセクション10「Craft Floor」は[impeccable](https://github.com/pbakaus/impeccable)から改変して採用している。そのプロジェクトの基準のうち一部のみを採用しており、ライセンスはApache-2.0である。設計プロセス全体を知りたい読者は、そのプロジェクトを参照できる。
