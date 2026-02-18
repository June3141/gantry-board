# Gantry Board - 開発ロードマップ

## Phase 0: 環境セットアップ ✅

### 成果物
- [x] Rust バックエンド hello world (axum /health)
- [x] React フロントエンド hello world (Vite)
- [x] プロジェクト設定 (.gitignore, LICENSE, CLAUDE.md)
- [x] プラグイン・スキル・サブエージェント導入
- [x] GitHub Actions CI
- [x] docker-compose.yml

### ドキュメント
- [x] PRD (docs/PRD.md)
- [x] ロードマップ (docs/ROADMAP.md)
- [x] プロジェクト指示 (CLAUDE.md)

---

## Phase 1: コアカンバン ✅

### 成果物
- [x] データモデル: projects, tasks, users (SQLite + sqlx)
- [x] Task CRUD API (REST)
- [x] カンバンボード UI (React + @dnd-kit)
- [x] ドラッグ&ドロップによるステータス変更
- [x] SSE によるリアルタイム同期

---

## Phase 2: 認証 + マルチユーザー ✅

### 成果物
- [x] セッションベース認証
- [x] ログイン / 登録 UI
- [x] API ミドルウェア認証
- [x] ユーザー管理

---

## Phase 3: エージェントオーケストレーション ✅

### 成果物
- [x] Git worktree CRUD (git2)
- [x] Claude Code CLI executor (tokio::process + stream-json)
- [x] Gemini CLI executor
- [x] エージェント出力 SSE ストリーミング
- [x] エージェント起動・停止 UI

---

## Phase 4: コメント / タイムライン ✅

### 成果物
- [x] コメントデータモデル + API
- [x] タスクタイムライン UI (コメント + エージェントセッション)
- [x] エージェント出力ビューア

---

## Phase 5: Docker プレビュー環境 ✅

### 成果物
- [x] Docker コンテナライフサイクル管理 (bollard)
- [x] ポート自動割り当て・衝突回避 (#198)
- [x] プレビュー URL 生成
- [x] Docker 操作 circuit breaker (#204)
- [x] プレビュー UI

---

## Phase 6: GitHub Projects 同期 ✅

### 成果物
- [x] GitHub OAuth + API 設定 (#143)
- [x] Issue 双方向同期 + Label マッピング (#144)
- [x] PR リンク検出 (#145)
- [x] GitHub 連携 UI (#146)
- [x] GitHub API キャッシュ層 (#195)
- [x] Webhook 受信 (#185)

---

## Phase 7: 仕上げ ← 進行中

### 成果物
- [x] エラーハンドリング強化 (#199, #207)
- [x] 構造化ログ管理 — Backend + Frontend pino (#200)
- [x] パフォーマンス最適化 — DB PRAGMA + 複合インデックス (#189, #194)
- [x] Service 層 trait 化 + DI (#201)
- [x] 大規模ファイル分割 (#191)
- [x] unwrap() 削除 (#193)
- [x] 本番 Docker 設定 (#186)
- [x] WebSocket fallback for SSE (#203)
- [x] ドキュメント整備 (#187, #205)

---

## セキュリティ・品質改善 (横断)

- [x] CORS origin 強制検証 (#206)
- [x] Host header バリデーション (#208)
- [x] Rate limiting (tower-governor)
- [x] npm audit fix (#192)
- [x] docker-compose セキュリティ設定 (#190)
- [x] Frontend Services 層 + エラーハンドラ (#196)
- [x] 正規化状態管理 (#202)
- [x] Agent 出力バッファリング (#197)
