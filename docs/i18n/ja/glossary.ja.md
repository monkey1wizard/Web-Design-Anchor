---
title: "用語集"
status: stable
updated: 2026-11-12
type: Reference
description: "WDAの仕様書とアーキテクチャ文書で使う用語を定義する。各項目は定義のみを示し、実装規則は示さない。"
tags: []
source: docs/glossary.md
lang: ja
source_sha256: 428a7cce5249c9b6e7eec0cdc58bd8e20d9b3155e1184ddd7b93550cf00766d9
translated_at: 2026-11-12
---

# 用語集

この文書では、WDAの仕様書とアーキテクチャ文書で使う用語を定義する。手順と検証ロジックについては、個別の文書を参照する。

## 役割と要素

| 用語 | 定義 |
| :--- | :--- |
| Designer | WDAの対象ユーザー。Designerは自然言語でデザインと機能の要件を伝え、コードを直接書かない |
| AI host | AIとの会話を実行し、Skillを読み込む外部環境。例としてClaude Codeがある。WDAはこの環境を信頼しない |
| Skill | AIの操作マニュアル兼ワークフローガイド。ファイルは`skills/wda/`にあり、各リリースに含まれる。仕様については`docs/skill-spec.md`を参照する |
| Core | WDAのCLIツール本体。`wda_core`ライブラリ（lib）と`wda`実行ファイル（bin）を含む単一のCargoパッケージ |
| Generated Project | `wda init`が作成し、DesignerとAIが長期にわたって保守するプロジェクトリポジトリ。契約については`docs/generated-project.md`を参照する |
| Standard Web Project | HTML5、Modern CSS、TypeScriptを使い、フロントエンドフレームワークに依存しないWebプロジェクト。Generated Projectはこの種のプロジェクトである |
| `dist/` | `wda build`が生成する唯一のデプロイ用ディレクトリ。単独で動作するプレーンな静的ファイルを格納する |
| Build Only | WDAが単独でデプロイできる静的ファイルのみをビルドするというアーキテクチャ上の取り決め。ホスティングやデプロイのサービスは提供しない |
| Process Contract | SkillとAI hostの機能に関する合意。ホストに必要なのは、ネイティブバイナリの実行と、stdout、stderr、終了ステータスコードの取得のみ |
| Lit | Web Componentsを構築するためのパッケージ。Spectrumはこれをウェブサイトの依存関係としてインストールし、Skillは後からこれを`WDA Minimal`プロジェクトに追加できる |

## 安定性レイヤー

| 用語 | 定義 |
| :--- | :--- |
| Invariant | どのバージョンでも変更する予定のない長期的な取り決め。たとえば、`dist/`が唯一のデプロイ成果物であること |
| Default | プロジェクトの既定値。`anchor.json`で変更できる |
| Tool Form | Coreツールの実行形態。現在は単独で動作するCLI実行ファイルである |
| Tooling Choice | Tool Formの実装に使う技術。たとえばRust、Rolldown、内蔵のTypeScriptツール。プロジェクト契約に影響しない範囲で、WDAは後のリリースで置き換えられる |
| Implementation Detail | Core内部のコード構成と実装。外部から直接見えるものではない |

## プロジェクト契約

| 用語 | 定義 |
| :--- | :--- |
| `anchor.json` | プロジェクトルートにあるWDAのメタデータファイル。WDAの設定、実際のDesign System、初期化のフォールバック記録を記録する。`schemas/anchor.schema.json`に照らして検証される |
| `package.json` | 現行のGenerated Projectでウェブサイト依存関係を宣言する唯一のファイル。依存関係の変更は外部パッケージマネージャーのnpmまたはBunが行う。Spectrumの初期化が成功すると、利用可能なマネージャーが一つだけでも、標準の`packageManager`フィールドに選んだマネージャーと正確なバージョンを記録する。初期状態のMinimalにはこのフィールドがない |
| External Package Manager | Designerの環境が提供するnpmまたはBunのツール。`package.json`の変更、そのlockfile、バージョン解決、推移的なバージョン、インストールを管理する。WDAはNode.jsやBunをインストールしない |
| Manager Lockfile | 外部パッケージマネージャーが書き込むlockfile。マネージャーが管理する。WDAはこれを書き込み、書き換え、移行せず、`wda check`と`wda build`はインストール済みの内容だけを読む |
| Tool Cache | WDAのリリースに内蔵された単一のTypeScriptツールの、現在のプラットフォーム向けのローカルコピー。`wda tools fetch`は引数なしで復元する。`anchor.json`と`package.json`は変更しない |
| Fallback | `anchor.json`の`initialization.fallback`の記録。`wda init`が意図したシステムの代わりに`WDA Minimal`を作成した場合に、`requestedDesignSystem`と`reason`を保持する。Spectrumの統合と検証が成功すると、Skillはこの記録を含む`initialization`を削除して`wda check`を再実行する。Litだけを追加した場合は記録を維持する |
| `wdaVersion` | `anchor.json`のフィールド。プロジェクトの処理に最後に成功したWDAのバージョンを記録する。フォーマット互換性の判定には使わない |
| Source Baseline | プロジェクトのソースコードで使用できる技術の範囲。HTML5とセマンティックマークアップ、Modern CSS、TypeScript、ブラウザーのネイティブ機能を含む |
| Deployment Baseline | `dist/`成果物に含められるファイル形式。HTML、CSS、JavaScript、静的アセットを含む |
| Baseline Widely Available | ブラウザーの既定の互換性基準。主要なブラウザーエンジンが少なくとも30か月サポートしているWebプラットフォーム機能を指す |
| WCAG 2.2 AA | Webコンテンツに対してプロジェクトが既定で採用するアクセシビリティ標準 |
| MPA | Multi-Page Application。各ページが個別のHTMLファイルを使い、ブラウザーのネイティブナビゲーションでページを切り替える。WDAプロジェクトの既定アーキテクチャ |
| SPA | Single-Page Application。クライアント側ルーティングでビューを動的に切り替える。WDAでは採用できるが、既定にはしない |
| Strict Schema | `anchor.json`で受け付けるのは宣言済みフィールドのみ。不明なフィールドはErrorとして報告される。独自の拡張領域やメタデータフィールドはない |
| Clean URL | URLからファイル拡張子を取り除く、またはネストした`index.html`ディレクトリでルートを模擬するパス形式。WDAはこの形式をサポートせず、デプロイ先のサーバーが処理する |

## ビルド

| 用語 | 定義 |
| :--- | :--- |
| `wda-include` | WDAがサポートする唯一の独自ビルド時HTMLタグ。ビルド時にコンポーネントの内容へ展開され、`dist/`には書き込まれない |
| Build Staging Directory | `wda build`が、`dist/`へ列挙済みのentry、chunk、静的アセットを書き出す前に、バンドラーの出力を入れる新しい一時ディレクトリ。プロジェクトの一部ではなく、ビルド終了時に削除される |
| Collision Rule | `wda init`の事前安全確認。コマンドが作成予定のパスに既存ファイルがある場合、ただちに実行を拒否する |
| Rollback Baseline | `wda init`がプロジェクトディレクトリに作成するローカルGitリポジトリの最初の初期コミット |
| Resolution Layer | CoreがDesign Systemごとにロジックを分岐する最初の許可対象インターフェース。`src/init/resolve.rs` |
| Built-in Asset Layer | CoreがDesign Systemごとにロジックを分岐する2番目の許可対象インターフェース。`src/init/`と`src/builtins/`にある指定ファイルで構成される |

## デザインシステム

| 用語 | 定義 |
| :--- | :--- |
| Design Token | セマンティックな識別子名で参照する最小のデザイン値。たとえば色、間隔、フォントサイズ |
| DTCG | Design Tokens Community Groupが定義する標準JSON形式。このプロジェクトでは`tokens/tokens.json`のみを権威ある情報源として使う |
| `WDA Minimal` | オフライン環境向けにWDAに組み込まれた最小のDesign System。初期化時にウェブサイトの依存関係を作成しないため、ネットワーク接続やパッケージマネージャーがなくても利用できる。Skillは、利用者の同意を得た後にSpectrumとLitを追加できる |
| Spectrum / Spectrum 2 | AdobeのDesign System。`wda init`がオンラインで問い合わせる際の既定の選択肢で、利用可能な外部パッケージマネージャーがインストールする |
| Adapter | Design SystemをWDAプロジェクトに接続する方法を定めるインターフェース仕様。7つのファセットで構成される |
| Facet | Adapterインターフェース仕様を構成する要素。Tokenのマッピング、スタイルの公開と整理、フォント、アイコンアセット、カラーテーマ、コンポーネント依存モデル、パッケージ統合がある |
| Neutrality | Coreを単一のDesign Systemに限定しないというアーキテクチャ上の取り決め。適格なシステムは異なる依存関係モデルとコンポーネントモデルをサポートし、それぞれがinit、check、buildを完了できなければならない |
| Identity Branch | Design System名を識別する、またはその名前に応じて分岐するCoreのロジック。許可対象ファイル内にのみ置ける |

## 検証と診断

| 用語 | 定義 |
| :--- | :--- |
| Diagnostic | `wda check`または`wda build`がプロジェクトを検査した際に出力する問題レコード。ソース位置がある場合、行番号とUTF-8バイト単位の列番号は1から始まる |
| Severity | Diagnosticの分類。ビルドを止めるError、または止めないWarningがある |
| Error | ビルド結果が誤っている、またはプロジェクト契約に違反していることを示す重大な問題。ビルドを止める。`ValidationReport`を通じて報告される場合、コマンドの終了ステータスコードは1 |
| Warning | ビルド出力を止めないが、潜在的なリスクを示してユーザーに注意を促す問題。人間向け出力ではstderrに書き込まれる |
| ToolFault | 外部依存関係ツールまたはファイルシステムが予期せず、または重大な形で失敗したときに返されるツールエラー。コマンドがToolFaultを返す場合、終了ステータスコードは2 |
| Exit Status | CLIコマンドの実行後に返されるステータス。`0`は成功またはWarningのみ、`1`は`ValidationReport`にErrorが含まれる場合、`2`は使用法エラーまたはToolFaultを示す |
| Single Validation Seam | Coreが公開する唯一の検証関数`validate_project(root)`。バイナリから個々の内部検証ルールモジュールを直接利用することはできない |
| Probe Test | 特定の実行環境の動作や機能が期待に合うか確認するテスト |

## Skillと受け入れ

| 用語 | 定義 |
| :--- | :--- |
| Allowlisted Content | `SKILL.md`に含められる6種類のガイダンス。役割の境界、バージョン比較の手順、判断ルーティング表、ワークフローの停止条件、報告義務、権威ある文書へのポインター |
| Undertriggering | AIがSkillを読み込むべき状況で読み込まず、Core仕様に従わずにファイルを編集すること |
| Four-Tier Acceptance | Skillの品質を確認するフレームワーク。L1仕様準拠、L2境界保護、L3動作評価、L4削除テストで構成される |
| Deletion Test | L4の受け入れ方法。Skill全体を削除し、Coreとプロジェクト文書だけでユーザーがWDAを正しく操作できることを確認する |
| Authority Pointer | Skillの規則を述べる文と同じ段落に置き、文書パスを示す参照マーカー。例として`docs/`、セクション記号`§`、`ADR-`がある |
| Craft Floor | Generated Projectのデザイン文書で、最低限の品質基準を定める節。デザイン成果物が満たすべき基本条件を列挙する |

## 文書

| 用語 | 定義 |
| :--- | :--- |
| Required Documents | Generated Projectの初期化時に必須となる4つのコア文書。`README.md`、`docs/architecture.md`、`docs/design.md`、`docs/glossary.md` |
| Conditional Documents | 他の文書で扱われていない特別なワークフローが現れた場合にのみGenerated Projectが作成する文書。例として`docs/development.md`がある |
| Canonical Source | 特定のプロジェクト状態または仕様を示す唯一の情報源。他の文書は要約のみでき、その内容を複製してはならない |
| `sourceRefs` | 文書のフロントマターにある配列フィールド。文書の基礎または派生元となる正規ソースのパスを列挙する |
| Front Matter | Markdown文書の冒頭にあるYAML形式の構造化メタデータ。`---`で囲む |
| ADR | Architecture Decision Record。単一の技術的決定について、その背景、選択肢、決定、結果を記録する |
| Thematic ADR | 関連するアーキテクチャ上の決定をトピックごとにまとめたADR文書 |
| OKF | Open Knowledge Format。フロントマターのフィールド構造と、このプロジェクトの文書標準の仕様基盤 |
| restore point | プロジェクトの状態を復元するために使えるバージョン管理上の記録点 |
| SEEN | Designerが現在の結果を確認したことを示すステータスマーカー |
| ACCEPTED | Designerが現在の結果を明示的に承認したことを示すステータスマーカー |
