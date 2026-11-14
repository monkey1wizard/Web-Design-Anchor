---
title: "Spectrum Adapter の対応状況"
status: stable
updated: 2026-11-12
type: Reference
description: "Spectrum 2 の実装と WDA Adapter の七つの facet の対応状況。"
tags: []
source: docs/design-systems/spectrum-adapter.md
lang: ja
source_sha256: a28518d6dfcc81b176e7a644821dbdf539b016c5a2b1fe9183f192802170151c
translated_at: 2026-11-12
---

# Spectrum Adapter の対応状況

この文書では、コードに実装されている Spectrum 2 の統合を記録する。七つの facet の一般的な定義は [docs/design-system.md](../../design-system.md) を参照する。

Spectrum 2 は 2026-09-07 にプロジェクトへ追加された。コミット履歴は `src/builtins/spectrum_two/` にある。

## 七つの facet

### 1. Token mapping

コードでは Spectrum token と WDA token の対応を定義していない。

### 2. Styles

ビルド時に `src/build/` の原生ビルドが、原生バンドラーでスクリプトのエントリーをバンドルする。その後、検証済みの原生 TypeScript ツールで型チェックを行う。Deno や Node.js の実行環境は不要である。
これはコード内のバンドル処理を説明するもので、Spectrum 専用のスタイル統合が別に存在するという意味ではない。

### 3. Fonts

初期依存関係の一覧にフォントパッケージはない。ビルドコードにもフォントアセットを解決する処理はない。

### 4. Icons

初期依存関係の一覧に独立したアイコンパッケージはない。ビルドコードにもアイコンアセットを解決する処理はない。

### 5. Theme

初期依存関係の一覧には `@spectrum-web-components/theme` が含まれる。コードでは、ほかのテーマ設定や切り替え動作を定義していない。

### 6. Component dependencies

スターターページは `@spectrum-web-components/*` パッケージを使う。下表にパッケージとほかの依存関係の統合方法を示す。

### 7. Dependency integration

`src/init/mod.rs` の `spectrum_two_dependencies` は、六つの Spectrum パッケージを同じ解決済み Spectrum バージョンに固定する。`lit` は独自に解決したバージョンに固定する。
初期化時にこれらの正確なバージョンがプロジェクトの `package.json` に書き込まれる。その後、プロジェクトのパッケージマネージャーがパッケージを `node_modules` にインストールし、ロックファイルを書き込む。
原生ビルドは、`scripts/main.ts` のベアインポートとサブパスインポートを、インストール済みパッケージから解決する。ビルド時にもパッケージを取得せず、ブラウザの実行時にも取得しない。

## 対応状況の概要

| Facet | 実装内容 |
| --- | --- |
| Token mapping | token の対応は未定義 |
| Styles | ビルド時に原生ビルドがスクリプトのエントリーをバンドル |
| Fonts | フォントパッケージもビルド時のフォント解決もない |
| Icons | 独立したアイコンパッケージもビルド時のアイコン解決もない |
| Theme | 依存関係の一覧に theme パッケージを含む |
| Component dependencies | Spectrum Web Components パッケージ |
| Dependency integration | `package.json` が正確なバージョンを宣言し、パッケージマネージャーがインストールしてロックファイルを書き込む |

| パッケージ | バージョンの参照元 |
| --- | --- |
| `@spectrum-web-components/theme` | 解決された Spectrum のバージョン |
| `@spectrum-web-components/button` | 解決された Spectrum のバージョン |
| `@spectrum-web-components/textfield` | 解決された Spectrum のバージョン |
| `@spectrum-web-components/picker` | 解決された Spectrum のバージョン |
| `@spectrum-web-components/card` | 解決された Spectrum のバージョン |
| `@spectrum-web-components/dialog` | 解決された Spectrum のバージョン |
| `lit` | 個別に解決 |

上記の六つの Spectrum 直接依存パッケージは、`src/init/mod.rs` の `SPECTRUM_TWO_DIRECT_DEPENDENCY_PACKAGES` に記載されている。`lit` は別の直接依存で、バージョンは独立している。

対応範囲の制限:

- スターターは、上記のパッケージと、それらがスターターのスクリプトに公開するモジュールだけを対象とする。
- すべての npm パッケージがブラウザで動作することは主張しない。原生ビルドがローカルファイルとしてバンドルできるパッケージだけが動作する。
- インストール済みのどのパッケージも提供しないベアインポートやサブパスインポートは、ビルドを失敗させる。そのエラーは発生元のソースファイルを示す。

初期化時のインストールに失敗した場合、WDA は作成済みの Spectrum 専用ファイルと依存関係設定を削除し、その後 WDA Minimal プロジェクトを作成する。
