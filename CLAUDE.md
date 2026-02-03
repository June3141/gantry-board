# Gantry Board - プロジェクト指示

## プロジェクト概要

AI コードエージェント (Claude Code, Gemini CLI 等) をオーケストレーションするカンバンボードアプリケーション。
セルフホスト前提、複数人利用、GitHub Projects 連携。

## 開発方針

### テスト駆動開発 (TDD)

- 原則としてテスト駆動開発で進める
- 期待される入出力に基づき、まずテストを作成する
- テストが正しいことを確認できた段階でコミットする
- その後、テストをパスさせる実装を進める
- 実装中はテストを変更せず、コードを修正し続ける

### Rust 規約

- `cargo clippy` の警告をゼロに保つ
- `cargo fmt` でフォーマット統一
- エラーハンドリングは `thiserror` でドメインエラー定義、`anyhow` はアプリケーション層のみ
- `unwrap()` / `expect()` はテストコードのみ。本番コードでは `?` で伝播
- モジュール構成は単一クレートで始め、必要に応じて Cargo workspace に分割

### React/TypeScript 規約

- ESLint + Prettier でフォーマット統一
- Vitest でテスト
- 状態管理は Zustand
- コンポーネントは関数コンポーネントのみ

## 技術スタック

| レイヤー | 技術 |
|---------|------|
| バックエンド | Rust (axum + tokio) |
| フロントエンド | React + TypeScript (Vite) |
| DB | SQLite (sqlx, WAL モード) |
| API ドキュメント | utoipa → OpenAPI JSON → orval (TypeScript client) |
| バリデーション | garde (Rust) |
| 状態管理 | Zustand (UI 状態) + TanStack Query (サーバ状態) |
| スタイリング | Tailwind CSS |
| リアルタイム | WebSocket (axum) |
| Docker 管理 | bollard |
| Git 操作 | git2 |
| エージェント実行 | tokio::process (CLI サブプロセス, stream-json) |
| テスト | cargo test / Vitest |
| タスクランナー | Taskfile (go-task) |

## エージェント連携

CLI サブプロセス方式 (Vibe Kanban と同じアーキテクチャ):

```
tokio::process::Command → claude -p --output-format=stream-json
                        → gemini --output-format stream-json
```

- CLI の設定ファイル (.claude/ 等) がそのまま共有される
- ユーザーの CLI セッションを gantry_board に取り込み可能

## ディレクトリ構成

```
backend/    — Rust バックエンド (単一クレート、成長に応じて分割)
frontend/   — React フロントエンド
docs/       — プロジェクトドキュメント
.claude/    — Claude Code 設定・エージェント・コマンド
```

## コミット規約

- コミットメッセージは英語
- `type: emoji subject` 形式: `feat: ✨ add health check endpoint`
- 1 コミット = 1 関心事、最大 10 ファイル / 300 行 (テスト・自動生成除く)
- TDD: テストコミット (✅) と実装コミット (✨/🐛) を分離
- 詳細は `.claude/skills/commit-rules/SKILL.md` を参照

## 品質自動化

3 層の Claude Code Hooks でコード品質を自動的に担保する:

| レイヤー | フックイベント | 内容 | ブロック |
|---------|-------------|------|---------|
| L1 | PostToolUse (Write/Edit) | フォーマッタ自動修正 (`cargo fmt` / `prettier --write`) | しない |
| L2 | Stop | 変更レイヤーの lint のみ実行 (`clippy` / `eslint`) | しない (通知) |
| L3 | PreToolUse (git commit) | コミットメッセージ検証 + コミットサイズ検証 + `task check` | **する** |

### エージェントが従うルール

- OpenAPI アノテーション変更後は `task api:generate` を実行する
- テスト実装後、`task backend:test` / `task frontend:test` で確認する
- コマンド一覧は `.claude/skills/quality-checks/SKILL.md` を参照
