---
title: "セキュリティポリシー"
status: stable
updated: 2026-11-12
type: Guide
description: "WDAのセキュリティモデル、サポート対象バージョン、脆弱性報告方法。"
tags: []
source: SECURITY.md
lang: ja
source_sha256: 6e262d027ce264ad2c6800163d5f62a102bb864128e275a5d488d7fbd964f272
translated_at: 2026-11-12
---

この文書は、WDAツールとその公開ソースコードの脆弱性報告手順を扱う。WDAで作成した個別のWebサイトや、そのデプロイ環境は対象外である。

## サポート対象バージョン

公開GitHub Releasesでは安定版`v0.1.3`とプレリリースを提供している。プレリリースはテスト版であり、変更される場合がある。バージョンとアセットは[GitHub Releases](https://github.com/monkey1wizard/Web-Design-Anchor/releases)を参照する。

`gal-last-good`は開発用タグであり、リリースではない。

すべてのセキュリティ修正は、非公開ソースリポジトリの`main`に入る。`docs/architecture.md`の「Version Compatibility」節にある4段階のフォーマット互換性レベルは、プロジェクト形式の互換性のみを示す。セキュリティサポートレベルを示すものではない。「Supported legacy format」はセキュリティサポート階層を意味しない。

## 脆弱性の報告

公開リポジトリでは現在、GitHubの非公開脆弱性報告を有効にしていないため、非公開報告窓口はない。脆弱性の詳細を公開Issueに書いてはならない。

報告にはベストエフォートで協調的な開示を行う。固定日数のSLAも、専用のメール報告窓口も設けていない。

## セキュリティモデル

この節では、実装済みのセキュリティ境界のみを示す。

規則の唯一の情報源は[`docs/architecture.md`](../../../docs/architecture.md)であり、ここでは繰り返さない。

### 保護対象

- プロジェクト契約、ソースツリー、デプロイ出力`dist/`。境界と完全性は`docs/generated-project.md`、`docs/commands.md`の「What It Creates」節および`docs/design-system.md`に従って保護する。
- `package.json`にあるWebサイトのパッケージ宣言、外部のパッケージマネージャーが管理するロックファイルと`node_modules`、ネイティブTypeScriptツールのキャッシュ。情報源と解決結果は`docs/commands.md`の「Website Packages」節と「wda build」節に従って保護する。
- Skillパッケージとホストプロセス契約。その責任境界は`docs/skill-spec.md`に従って保護する。

### 信頼する役割

- Designerはプロジェクトソースとデザイン上の決定を担う。AIとSkillは権威ある文書を経由して動作する協力者である。Core CLIは検証、初期化、依存関係、ビルドを実行するツールである。役割分担は`docs/architecture.md`、`docs/commands.md`、`docs/skill-spec.md`の「Core Principles」節を参照する。
- AI hostはSkillを実行する外部環境である。WDAはこれを信頼済みの実行環境とは見なさない。想定する機能はネイティブバイナリの実行と、stdout、stderr、終了ステータスの取得に限られる。`docs/skill-spec.md`を参照する。

### 信頼境界

- Designer、AI、Skillは、CLIの責務と出力契約によってWDA Core CLIから分離される。`docs/commands.md`の「Diagnostic Output」節を参照する。
- ソース、生成された`dist/`、デプロイ環境は別々のツリーと責任境界である。`docs/generated-project.md`を参照する。
- SkillとAI hostの間には、インストール済みコピーとプロセス契約の境界がある。`scripts/`と`assets/`はSkill製品の一部ではない。`docs/skill-spec.md`を参照する。
- 非公開GitHubソースリポジトリと公開GitHubリリースリポジトリは、異なる2つの信頼境界である。公開リポジトリには親を持たないキュレーション済みスナップショットを送る。エクスポートプログラムは非公開コンテンツを除外し、残存物をスキャンする。このスナップショットは非公開リポジトリのミラーではない。
- 非公開コントローラーは、必須のLinuxおよびWindowsチェックが成功した後にのみスナップショットを公開する。公開`main`と対応するバージョンタグは、1回のアトミックなGit pushで同じスナップショットコミットを指さなければならない。既存のバージョンタグを上書きしてはならない。
- リポジトリ間の認証情報は、公開ステップの一時的なaskpassヘルパーにのみ渡す。認証情報をリモートURL、キュレーション済みスナップショット、リポジトリ設定に書き込んではならない。
- 公開リリースワークフローは、公開リポジトリのバージョンタグのみを受け付ける。6つのターゲットをビルドし、チェックサムとcosign署名を生成してからGitHub Releaseを作成する。HomebrewとWinGetの公開ステージを実行するのは安定版リリースのみである。
- TypeScriptツールは、各WDAリリースに組み込まれた単一の公式バージョンであり、プラットフォームごとのパッケージは`sha512`のSRIに結び付けられている。組み込まれたSRIがツールの唯一の信頼の根拠であり、プロジェクトが上書きすることはできない。キャッシュの項目は展開の前にこのSRIで検証され、ビルドがツールをダウンロードすることはない。
- Webサイトのパッケージのインストールは、npmやBunなどの外部パッケージマネージャーを通じてサードパーティーのコードを実行する。この手順は`wda check`および`wda build`とは分けられている。後者の2つはパッケージをインストールせず、ネットワークも使わない。WDAは、サードパーティーのインストールスクリプトの副作用を元に戻せるとは約束しない。

### 不変条件

- `docs/architecture.md`の「Stability Layers」節に定義された長期契約のみを受け入れる。デプロイ出力は`dist/`のみであり、`docs/generated-project.md`に従う。
- `anchor.json`は厳密なWDA設定の契約である。`package.json`がWebサイトのパッケージを宣言する唯一の場所であり、ロックファイルは外部のパッケージマネージャーが管理する。WDAがバージョンを解決することはない。`docs/commands.md`の「What It Creates」節と「Website Packages」節を参照する。
- Skillは第2の仕様情報源にならず、`scripts/`は作成しない。そのホスト機能はネイティブバイナリのプロセス契約に限られる。`docs/skill-spec.md`を参照する。

### 攻撃対象領域

- 信頼されない入力のパーサーは、`Cargo.toml`の`jsonschema`、`json-sourcemap`、`marked-yaml`、`html5gum`、`serde_json`の依存関係である。それぞれスキーマとJSON、YAMLフロントマター、HTML入力を扱う。「Diagnostic Output」節、`docs/commands.md`の「What It Creates」節、文書とソースの境界に対応する。
- `serde_json`は`anchor.json`、`package.json`、生成された`tokens.json`を直接解析する。これらの入力は`docs/commands.md`の「What It Creates」節と「Website Packages」節、および`docs/design-system.md`に含まれる。
- `wda init`は`.git/`、`.gitignore`、`README.md`、`anchor.json`、`package.json`、`docs/architecture.md`、`docs/design.md`、`docs/glossary.md`、`pages/index.html`、`tokens/tokens.json`を書き込む。Spectrum 2経路では`scripts/main.ts`も書き込む。`init`が実行したパッケージマネージャーだけが、独自のロックファイルと`node_modules`を書き込む。衝突、ロールバック、ローカルGitの動作は`docs/commands.md`の「wda init」節に従う。
- 外部のパッケージマネージャー（npmやBunなど）が実行されるのは、`wda init`が既定のパッケージをインストールするときだけである。マネージャーはバージョンを解決し、独自のライフサイクルスクリプトを実行する。WDAは独自のパッケージリゾルバーを実行しない。
- Spectrumのオンライン解決は、利用者が指定した外部のパッケージマネージャーを通じて、レジストリから正確なバージョンを解決する。Design Systemの情報源とオフライン時のフォールバックは`docs/design-system.md`を参照する。
- Skill hostの機能は、ネイティブバイナリの実行とstdout、stderr、終了ステータスの取得に限られる。`scripts/`は空であり、作成してはならない。`docs/skill-spec.md`を参照する。

## セキュリティ検証の限界

各GitHub Releaseのビルド、署名、検証結果は、そのReleaseにリンクされたActions実行記録とアセットで確認する。プレリリースではHomebrew公開やWinGet提出を実行しない。これらのパッケージマネージャー経路は安定版リリースプロセスに属する。

非公開ソースリポジトリはGitHub Freeと単独メンテナーの信頼モデルを採用する。非公開ワークフローとそのSecretsを変更できるメンテナーは、公開を許可することもできる。GitLabバックアップの保守方針は、非公開の保守手順で管理する。
