---
title: "セキュリティポリシー"
status: stable
updated: 2026-10-23
type: Guide
description: "WDAのセキュリティモデル、サポート対象バージョン、脆弱性報告方法。"
tags: []
source: SECURITY.md
lang: ja
source_sha256: 966ae0299fff14cf06166de38c8dfe2a5caa63d5c8f44424a1a8635f36085134
translated_at: 2026-10-23
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

- プロジェクト契約、ソースツリー、デプロイ出力`dist/`。境界と完全性は`docs/generated-project.md`の「`wda.json`」節および`docs/design-system.md`に従って保護する。
- 依存関係の宣言、ロック状態、ビルド時の構成。情報源と解決結果は`docs/commands.md`の「wda build」節と「wda deps」節に従って保護する。
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

### 不変条件

- `docs/architecture.md`の「Stability Layers」節に定義された長期契約のみを受け入れる。デプロイ出力は`dist/`のみであり、`docs/generated-project.md`に従う。
- `wda.json`は厳密なプロジェクト契約である。依存関係グラフは`deno.jsonc`で宣言し、`deno.lock`でロックする。`docs/generated-project.md`の「`wda.json`」節と`docs/commands.md`の「wda deps」節を参照する。
- Skillは第2の仕様情報源にならず、`scripts/`は作成しない。そのホスト機能はネイティブバイナリのプロセス契約に限られる。`docs/skill-spec.md`を参照する。

### 攻撃対象領域

- 信頼されない入力のパーサーは`Cargo.toml` 16–21行目の`jsonschema`、`json-sourcemap`、`jsonc-parser`、`marked-yaml`、`html5gum`、`serde_json`である。それぞれスキーマとJSON、YAMLフロントマター、HTML入力を扱う。「Diagnostic Output」節、`docs/generated-project.md`の「`wda.json`」節、文書とソースの境界に対応する。
- `serde_json`は`wda.json`、生成された`tokens.json`、`deno.lock`を直接解析する。これらの入力はそれぞれ`docs/generated-project.md`の「`wda.json`」節、`docs/design-system.md`、`docs/commands.md`の「wda deps」節に含まれる。
- `wda init`は`.git/`、`.gitignore`、`README.md`、`wda.json`、`docs/architecture.md`、`docs/design.md`、`docs/glossary.md`、`pages/index.html`、`tokens/tokens.json`を書き込む。Spectrum 2経路では`scripts/main.ts`、`deno.jsonc`、`deno.lock`も書き込む。WDA Minimal経路ではLitのインストール成功時のみ`deno.jsonc`と`deno.lock`を書き込む。衝突、ロールバック、ローカルGitの動作は`docs/generated-project.md`と`docs/commands.md`に従う。
- `deno run -A npm:esbuild@…`はツールをダウンロードし、全権限のサブプロセスを起動する。これはP5とP6で実装されたビルド時の境界である。責務は`docs/architecture.md`の「Stability Layers」節と`docs/commands.md`の「wda deps」節に従う。
- Spectrumのオンライン解決は、Denoを介してnpmレジストリから正確なバージョンを解決する。Design Systemの情報源とオフライン時のフォールバックは`docs/design-system.md`を参照する。
- Skill hostの機能は、ネイティブバイナリの実行とstdout、stderr、終了ステータスの取得に限られる。`scripts/`は空であり、作成してはならない。`docs/skill-spec.md`を参照する。

## セキュリティ検証の限界

各GitHub Releaseのビルド、署名、検証結果は、そのReleaseにリンクされたActions実行記録とアセットで確認する。プレリリースではHomebrew公開やWinGet提出を実行しない。これらのパッケージマネージャー経路は安定版リリースプロセスに属する。

非公開ソースリポジトリはGitHub Freeと単独メンテナーの信頼モデルを採用する。非公開ワークフローとそのSecretsを変更できるメンテナーは、公開を許可することもできる。GitLabバックアップの保守方針は、非公開の保守手順で管理する。
