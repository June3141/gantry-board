# Gantry Board - プロダクト要件定義書 (PRD)

## ビジョン

AI コードエージェントを活用したソフトウェア開発を、チームで効率的に管理・可視化するプラットフォーム。
Vibe Kanban のようなカンバン管理に加え、チーム議論・GitHub 連携・Docker プレビュー環境を統合する。

## ターゲットユーザー

- AI コーディングエージェント (Claude Code, Gemini CLI 等) を活用する開発チーム
- セルフホスト環境で複数人が協働する組織

## コア要件

### 1. カンバンボード (タスク管理)

- ドラッグ&ドロップによるステータス変更 (Backlog → Todo → In Progress → In Review → Done)
- タスクの優先度設定 (Low / Medium / High / Urgent)
- 親タスク・サブタスクの階層構造
- プロジェクト単位でのボード管理

### 2. エージェントオーケストレーション

- タスクに AI エージェント (Claude Code / Gemini CLI) を割り当て
- エージェント出力のリアルタイムストリーミング (WebSocket)
- セッション管理: 開始・停止・一時停止・再開
- CLI との完全互換: ユーザーが CLI で直接行った作業のセッションも取り込み可能
- 複数エージェントの並列実行

### 3. Git Worktree 管理

- タスクごとに独立した git worktree を自動作成
- ブランチの自動命名 (`task/{task-id}`)
- worktree のライフサイクル管理 (作成・一覧・削除)
- マージ後の自動クリーンアップ

### 4. Docker プレビュー環境

- worktree ごとにオンデマンドで Docker コンテナを起動
- ポート管理: 自動割り当て、衝突回避
- プレビュー URL の生成
- コンテナのライフサイクル管理

### 5. ユーザー議論 / チャット

- タスクごとのスレッド形式ディスカッション
- プロジェクト全体のチャットチャンネル
- エージェント出力とユーザーコメントの統合タイムライン
- リアルタイム配信 (WebSocket)

### 6. GitHub Projects 連携

- GitHub Projects V2 との双方向同期
- Issue / PR との自動紐付け
- Webhook によるリアルタイム同期
- ラベル・マイルストーンの同期

### 7. マルチユーザー / 認証

- セッションベース認証
- ユーザー管理 (招待・ロール)
- タスクのアサイン

## アーキテクチャ概要

```
┌──────────────────────────┐
│   Frontend (React/Vite)  │
│   カンバン│チャット│モニター  │
└───────────┬──────────────┘
            │ WebSocket + REST
┌───────────┴──────────────┐
│   Backend (Rust/axum)    │
│ ┌────────┐ ┌───────────┐ │
│ │ Task   │ │ Agent     │ │
│ │ Service│ │ Orchestr. │ │
│ └───┬────┘ └─────┬─────┘ │
│     │      ┌─────┴─────┐ │
│     │      │ Worktree  │ │
│     │      │ Manager   │ │
│     │      └─────┬─────┘ │
│ ┌───┴────┐ ┌─────┴─────┐ │
│ │ SQLite │ │ Docker    │ │
│ │ (sqlx) │ │ (bollard) │ │
│ └────────┘ └───────────┘ │
└──────────────────────────┘
         │           │
    ┌────┴────┐ ┌────┴────┐
    │Claude   │ │Docker   │
    │Code CLI │ │Containers│
    │Gemini   │ │(preview) │
    └─────────┘ └─────────┘
```

## エージェント連携方式

Vibe Kanban と同じ CLI サブプロセス方式:

- `tokio::process::Command` で CLI を子プロセス起動
- `--output-format=stream-json` で stdin/stdout パイプ通信
- `--resume SESSION_ID` でセッション再開
- `kill_on_drop(true)` + `CancellationToken` でプロセス管理

### メリット

- CLI とのシームレスな互換性 (設定・セッション共有)
- ユーザーはいつでも CLI に戻れる「脱出口」がある
- 新しいエージェント追加が容易 (CLI があれば対応可能)

## 依存 OSS ライセンス

| ツール/ライブラリ | ライセンス | 用途 |
|-----------------|-----------|------|
| axum | MIT | Web フレームワーク |
| tokio | MIT | 非同期ランタイム |
| serde | MIT / Apache-2.0 | シリアライズ |
| sqlx | MIT / Apache-2.0 | DB アクセス |
| bollard | Apache-2.0 | Docker API |
| git2 (libgit2) | MIT / GPL-2.0 (dual, MIT 選択) | Git 操作 |
| React | MIT | フロントエンド |
| Vite | MIT | ビルドツール |
| Gemini CLI | Apache-2.0 | エージェント実行 |
| Claude Code CLI | Anthropic 利用規約 | エージェント実行 |
| Vercel Agent Skills | MIT | フロントエンドスキル |
| awesome-agent-skills | MIT | エージェントスキル参照 |
| awesome-claude-code-subagents | MIT | サブエージェント参照 |

AGPL 依存なし。Daytona (AGPL-3.0) は不採用とした。

## 非機能要件

- **セルフホスト**: Docker Compose で一発起動可能
- **パフォーマンス**: WebSocket による低遅延リアルタイム通信
- **拡張性**: 新エージェント追加は CLI ラッパーの実装のみで完結
- **データ**: SQLite WAL モードで並行アクセス対応。将来的な PostgreSQL 移行パスを確保
