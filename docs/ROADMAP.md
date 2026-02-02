# Gantry Board - 開発ロードマップ

## Phase 0: 環境セットアップ ← 現在

### 成果物
- [x] Rust バックエンド hello world (axum /health)
- [ ] React フロントエンド hello world (Vite)
- [x] プロジェクト設定 (.gitignore, LICENSE, CLAUDE.md)
- [ ] プラグイン・スキル・サブエージェント導入
- [ ] GitHub Actions CI
- [ ] docker-compose.yml

### ドキュメント
- [x] PRD (docs/PRD.md)
- [x] ロードマップ (docs/ROADMAP.md)
- [x] プロジェクト指示 (CLAUDE.md)
- [ ] アーキテクチャ詳細 (docs/ARCHITECTURE.md)

---

## Phase 1: コアカンバン

### 成果物
- データモデル: projects, tasks, users (SQLite + sqlx)
- Task CRUD API (REST)
- カンバンボード UI (React + @dnd-kit)
- ドラッグ&ドロップによるステータス変更
- WebSocket によるリアルタイム同期

### 依存
- Phase 0 完了

---

## Phase 2: 認証 + マルチユーザー

### 成果物
- セッションベース認証
- ログイン / 登録 UI
- API ミドルウェア認証
- ユーザー管理

### 依存
- Phase 1 完了

---

## Phase 3: エージェントオーケストレーション

### 成果物
- Git worktree CRUD (git2)
- Claude Code CLI executor (tokio::process + stream-json)
- Gemini CLI executor
- エージェント出力 WebSocket ストリーミング
- エージェント起動・停止・再開 UI

### 依存
- Phase 1 完了 (タスクモデル)
- Phase 2 推奨 (認証)

---

## Phase 4: チャット / ディスカッション

### 成果物
- チャットデータモデル + API
- WebSocket チャットハンドラ
- タスクスレッド + プロジェクトチャンネル UI
- エージェント出力との統合タイムライン

### 依存
- Phase 1 完了
- Phase 2 完了

---

## Phase 5: Docker プレビュー環境

### 成果物
- Docker コンテナライフサイクル管理 (bollard)
- ポート自動割り当て・衝突回避
- プレビュー URL 生成
- Worktree 変更検知 → 自動プレビュー更新
- プレビュー UI (iframe or リンク)

### 依存
- Phase 3 完了 (worktree)

---

## Phase 6: GitHub Projects 同期

### 成果物
- GitHub OAuth + API 設定
- GitHub Projects V2 GraphQL 統合
- Issue / PR 双方向同期
- Webhook 受信

### 依存
- Phase 1 完了
- Phase 2 完了

---

## Phase 7: 仕上げ

### 成果物
- エラーハンドリング強化
- ログ管理
- 本番 Docker 設定 (docker-compose.prod.yml)
- パフォーマンス最適化
- ドキュメント整備

### 依存
- Phase 1〜6 完了
