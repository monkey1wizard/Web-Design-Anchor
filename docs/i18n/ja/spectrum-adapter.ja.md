---
title: "Spectrum Adapter の対応状況"
status: stable
updated: 2026-10-23
type: Reference
description: "Spectrum 2 の実装と WDA Adapter の七つの facet の対応状況。"
tags: []
source: docs/design-systems/spectrum-adapter.md
lang: ja
source_sha256: 5fff8c565cd832fe736f3f4fb67568783beeb78d3f0f727359fc4f1866e03131
translated_at: 2026-10-23
---

# Spectrum Adapter の対応状況

この文書では、コードに実装されている Spectrum 2 の統合を記録する。七つの facet の一般的な定義は [docs/design-system.md](../../design-system.md) を参照する。

Spectrum 2 は 2026-09-07 にプロジェクトへ追加された。コミット履歴は `src/builtins/spectrum_two/` にある。

## 七つの facet

### 1. Token mapping

コードでは Spectrum token と WDA token の対応を定義していない。

### 2. Styles

ビルド時に `src/build/scripts.rs` が Deno 経由で esbuild を実行する。このコマンドは `--bundle` を使ってスクリプトのエントリーをバンドルする。
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

`src/init/mod.rs::spectrum_two_dependency_specs` は、解決された Spectrum のバージョンに合わせてパッケージのバージョンを固定する。
初期化時に一覧がプロジェクトの `deno.jsonc` に書き込まれ、その後 `deno.lock` がバージョンをロックする。

## 対応状況の概要

| Facet | 実装内容 |
| --- | --- |
| Token mapping | token の対応は未定義 |
| Styles | ビルド時に esbuild `--bundle` でスクリプトのエントリーをバンドル |
| Fonts | フォントパッケージもビルド時のフォント解決もない |
| Icons | 独立したアイコンパッケージもビルド時のアイコン解決もない |
| Theme | 依存関係の一覧に theme パッケージを含む |
| Component dependencies | Spectrum Web Components パッケージ |
| Dependency integration | `deno.jsonc` が依存関係を宣言し、`deno.lock` がバージョンをロックする |

| パッケージ | バージョンの参照元 |
| --- | --- |
| `@spectrum-web-components/theme` | 解決された Spectrum のバージョン |
| `@spectrum-web-components/button` | 解決された Spectrum のバージョン |
| `@spectrum-web-components/textfield` | 解決された Spectrum のバージョン |
| `@spectrum-web-components/picker` | 解決された Spectrum のバージョン |
| `@spectrum-web-components/card` | 解決された Spectrum のバージョン |
| `@spectrum-web-components/dialog` | 解決された Spectrum のバージョン |
| `lit` | 個別に解決 |

上記の直接依存パッケージ七つは `src/init/mod.rs::spectrum_two_dependency_specs` に記載されている。
初期化時のインストールに失敗した場合、WDA は作成済みの Spectrum 専用ファイルと依存関係設定を削除し、その後 WDA Minimal プロジェクトを作成する。
