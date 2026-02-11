# Gantry Board — Architecture

AI コードエージェント (Claude Code, Gemini CLI) をオーケストレーションするカンバンボードアプリケーション。

## System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│  Frontend (React 19 + TypeScript, Vite)                        │
│  ┌──────────┐  ┌────────────┐  ┌───────────┐  ┌────────────┐  │
│  │ Kanban   │  │ Agent      │  │ Zustand   │  │ TanStack   │  │
│  │ Board    │  │ Panel      │  │ Stores    │  │ Query      │  │
│  └────┬─────┘  └─────┬──────┘  └───────────┘  └─────┬──────┘  │
│       │              │    SSE (EventSource)           │         │
│       └──────────────┼───────────────────────────────┘         │
└──────────────────────┼─────────────────────────────────────────┘
                       │ HTTP (fetch, credentials: include)
┌──────────────────────┼─────────────────────────────────────────┐
│  Backend (Rust, axum + tokio)                                  │
│  ┌───────────┐  ┌────┴──────┐  ┌───────────┐  ┌────────────┐  │
│  │ Handlers  │  │ SSE Hub   │  │ Services  │  │ Auth       │  │
│  │ (routes)  │  │ (bcast)   │  │ (logic)   │  │ Middleware │  │
│  └─────┬─────┘  └───────────┘  └─────┬─────┘  └────────────┘  │
│        │                              │                        │
│  ┌─────┴──────────────────────────────┴─────┐                  │
│  │  Agent Orchestrator                      │                  │
│  │  ┌──────────────┐  ┌──────────────────┐  │                  │
│  │  │ Claude Code  │  │ Gemini CLI       │  │                  │
│  │  │ Executor     │  │ Executor         │  │                  │
│  │  └──────┬───────┘  └────────┬─────────┘  │                  │
│  └─────────┼───────────────────┼────────────┘                  │
│            │ tokio::process    │                                │
│  ┌─────────┴───────────────────┴────────────┐                  │
│  │  SQLite (WAL mode, sqlx)                 │                  │
│  └──────────────────────────────────────────┘                  │
└────────────────────────────────────────────────────────────────┘
```

## Backend Module Structure

```
backend/src/
├── main.rs              # エントリポイント: Tokio runtime, listener, cleanup task
├── lib.rs               # AppState, Router 定義, CORS/rate-limit layers
├── config.rs            # 環境設定 (figment + dotenvy)
├── db.rs                # SQLite pool 初期化 (WAL mode)
├── error.rs             # AppError (thiserror), AppResult<T>
├── openapi.rs           # utoipa OpenAPI spec 定義
│
├── models/
│   ├── task.rs                  # Task, TaskStatus, TaskPriority
│   ├── project.rs               # Project, ProjectMember
│   ├── user.rs                  # User, RegisterRequest, LoginRequest
│   ├── agent_session.rs         # AgentSession, AgentType, AgentSessionStatus
│   └── agent_session_output.rs  # AgentSessionOutput (sequence 付き出力行)
│
├── handlers/
│   ├── health.rs            # GET /health
│   ├── auth.rs              # POST register/login/logout, GET me
│   ├── tasks.rs             # CRUD /tasks
│   ├── projects.rs          # CRUD /projects
│   ├── project_members.rs   # CRUD /projects/{id}/members
│   └── agent_sessions.rs    # start/stop/list/get/outputs
│
├── services/
│   ├── task_service.rs                  # Task CRUD + SSE broadcast
│   ├── project_service.rs               # Project CRUD
│   ├── member_service.rs                # ProjectMember CRUD
│   ├── user_service.rs                  # ユーザー登録・パスワード検証
│   ├── session_service.rs               # HTTP セッション管理 (cookie)
│   ├── authorization_service.rs         # プロジェクトメンバー権限チェック
│   ├── agent_session_service.rs         # AgentSession CRUD + ステータス遷移
│   ├── agent_session_output_service.rs  # 出力行 append/fetch
│   └── worktree_service.rs             # Git worktree 操作 (git2)
│
├── agent/
│   ├── executor.rs       # AgentExecutor trait, AgentHandle, AgentConfig
│   ├── orchestrator.rs   # セッション lifecycle 管理, background monitor
│   ├── claude_code.rs    # Claude Code CLI (stream-json NDJSON パース)
│   └── gemini_cli.rs     # Gemini CLI executor
│
├── sse/
│   ├── hub.rs      # SseHub: broadcast channel (capacity=256)
│   ├── handler.rs  # GET /api/events: SSE endpoint
│   └── event.rs    # SseEvent enum (task/agent イベント型)
│
├── auth/
│   ├── middleware.rs  # AuthUser extractor, cookie 生成/削除
│   └── password.rs    # Argon2 ハッシング, zxcvbn 強度チェック
│
└── test_helpers.rs    # テスト用 server/pool 生成ユーティリティ
```

### Layer Architecture

```
HTTP Request
    ↓
[Rate Limiting]  tower_governor (per-IP token bucket)
    ↓
[CORS]           tower_http CorsLayer
    ↓
[Tracing]        tower_http TraceLayer
    ↓
[Handler]        handlers::* (utoipa 注釈付き)
    ↓
[AuthUser]       auth::middleware (cookie → session → user)
    ↓
[Service]        services::* (ビジネスロジック + sqlx)
    ↓
[DB]             SQLite (WAL mode)
```

## Frontend Component Hierarchy

```
frontend/src/
├── main.tsx        # ReactDOM.createRoot
├── App.tsx         # BrowserRouter, QueryClientProvider
│
├── components/
│   ├── ProtectedRoute.tsx       # 認証ガード (authStore チェック)
│   ├── LoginPage.tsx            # ログインフォーム
│   ├── RegisterPage.tsx         # 登録フォーム
│   ├── KanbanBoard.tsx          # メインボード (@dnd-kit drag-drop)
│   ├── KanbanColumn.tsx         # ステータス別カラム
│   ├── TaskCard.tsx             # タスクカード
│   ├── TaskDetailModal.tsx      # タスク詳細 (AgentPanel 含む)
│   ├── TaskCreateDialog.tsx     # タスク作成ダイアログ
│   ├── ProjectCreateDialog.tsx  # プロジェクト作成ダイアログ
│   ├── AgentPanel.tsx           # エージェント start/stop UI
│   └── AgentOutputViewer.tsx    # セッション出力表示 (scrollable)
│
├── hooks/
│   ├── useEventSource.ts    # SSE 接続、TanStack Query 無効化
│   └── useAgentEvents.ts    # agent_output イベント → agentStore
│
├── stores/
│   ├── authStore.ts    # 認証状態 (persist to localStorage)
│   ├── agentStore.ts   # セッション出力バッファ (max 1000 lines)
│   ├── boardStore.ts   # ボード UI 状態
│   └── uiStore.ts      # グローバルダイアログ状態
│
└── api/
    ├── client.ts        # customInstance (fetch wrapper, credentials: include)
    └── generated/       # orval 自動生成
        ├── endpoints/   # TanStack Query hooks (tags-split)
        └── model/       # TypeScript 型定義
```

### Component Tree

```
App
└── BrowserRouter
    └── Routes
        ├── /login    → LoginPage
        ├── /register → RegisterPage
        └── /         → ProtectedRoute
            └── KanbanBoard
                ├── DndContext (drag-drop)
                │   └── KanbanColumn[] (Backlog/Todo/InProgress/InReview/Done)
                │       └── TaskCard[]
                ├── ProjectCreateDialog
                ├── TaskCreateDialog
                └── TaskDetailModal
                    ├── Task editor
                    ├── AgentPanel (start/stop controls)
                    └── AgentOutputViewer (streaming output)
```

## Data Flow

### Request → Response (例: タスク作成)

```
[Frontend]                         [Backend]                        [DB]
TaskCreateDialog                   handlers::tasks::create_task
   │                                  │
   ├─ useCreateTask()                 ├─ AuthUser extractor
   │  (orval generated hook)          │  (cookie → session → user)
   │                                  │
   ├─ POST /api/tasks                 ├─ garde バリデーション
   │  { title, project_id, ... }      │
   │  credentials: include            ├─ authorization_service
   │                                  │  ::require_project_member()
   │                                  │
   │                                  ├─ task_service::create_task() ──→ INSERT INTO tasks
   │                                  │
   │                                  ├─ sse_hub.broadcast(TaskCreated)
   │                                  │
   ← 201 Created { task }  ──────────┘
   │
   ├─ onSuccess: invalidateQueries(['/api/tasks'])
   │
   └─ UI re-render
```

### SSE Real-Time Updates

```
[Backend]                              [Frontend]
sse::hub::SseHub                       useEventSource.ts
  │ broadcast channel (cap=256)          │ new EventSource('/api/events')
  │                                      │
  ├─ SseEvent::TaskCreated ──────────→  addEventListener('task_created')
  │                                      └─ invalidateQueries(['/api/tasks'])
  │
  ├─ SseEvent::AgentOutput ──────────→  addEventListener('agent_output')
  │                                      └─ agentStore.appendOutput(text)
  │
  └─ SseEvent::AgentSessionStatusChanged → addEventListener('agent_session_status_changed')
                                           └─ invalidateQueries(['.../sessions'])
```

SSE イベント型一覧:

| Event | Payload | Trigger |
|-------|---------|---------|
| `task_created` | `Task` | タスク作成後 |
| `task_updated` | `Task` | タスク更新後 |
| `task_deleted` | `task_id` | タスク削除後 |
| `agent_output` | `session_id, text` | エージェント出力受信時 |
| `agent_session_status_changed` | `AgentSession` | ステータス変更時 |

## Agent Orchestration

### Session Lifecycle

```
POST /tasks/{id}/sessions/start
         │
         ▼
    ┌────────────────┐
    │ Orchestrator   │
    │ start_session  │
    └───────┬────────┘
            │
    ① Per-task lock 取得 (同時実行防止)
            │
    ② Executor 存在確認 (claude_code / gemini_cli)
            │
    ③ DB: 既存 pending/running セッション確認
       └─ 存在する場合 → 409 Conflict
            │
    ④ DB: INSERT agent_sessions (status=pending)
            │
    ⑤ worktree_service::create_worktree()
       └─ git2: ブランチ作成 + worktree 登録
            │
    ⑥ executor.start(AgentConfig) → AgentHandle
       └─ tokio::process::Command (CLI サブプロセス)
            │
    ⑦ DB: UPDATE status=running, started_at=now()
            │
    ⑧ Background monitor task (tokio::spawn)
       │
       ├─ AgentOutputEvent::Output { text }
       │   ├─ DB: INSERT agent_session_outputs
       │   └─ SSE: broadcast AgentOutput
       │
       ├─ AgentOutputEvent::Completed
       │   ├─ DB: UPDATE status=completed
       │   └─ SSE: broadcast AgentSessionStatusChanged
       │
       └─ AgentOutputEvent::Failed { error }
           ├─ DB: UPDATE status=failed
           └─ SSE: broadcast AgentSessionStatusChanged
```

### CLI Subprocess Execution

```
Claude Code:
  Command: claude -p --output-format=stream-json --include-partial-messages [--allowedTools ...]
  stdin:   <prompt>
  stdout:  NDJSON stream → parse_stream_line()
           ├─ { type: "stream_event", event: { delta: { text } } } → Output
           ├─ { type: "result", is_error: false }                   → Completed
           └─ { type: "result", is_error: true }                    → Failed

Gemini CLI:
  Command: gemini --output-format stream-json
  stdin:   <prompt>
  stdout:  NDJSON stream (similar parsing)
```

### Concurrency Control

1. **Memory lock**: per-task `Arc<Mutex<()>>` で start_session 呼び出しを直列化
2. **Runtime check**: orchestrator が DB 上のアクティブセッション有無を確認し、存在すれば 409 エラーを返す

## Database Schema

```
┌─────────────┐      ┌──────────────────┐      ┌────────────┐
│   users      │      │ project_members  │      │  projects  │
├─────────────┤      ├──────────────────┤      ├────────────┤
│ id (PK)     │◄─────│ user_id (FK)     │      │ id (PK)    │
│ email (UQ)  │      │ project_id (FK)  │─────►│ name       │
│ name        │      │ role             │      │ description│
│ password_   │      │ (UQ: proj+user)  │      └──────┬─────┘
│   hash      │      └──────────────────┘             │
└──────┬──────┘                                       │
       │                                              │
┌──────┴──────┐                              ┌────────┴────────┐
│  sessions   │                              │     tasks       │
│  (HTTP)     │                              ├─────────────────┤
├─────────────┤                              │ id (PK)         │
│ id (PK)     │                              │ project_id (FK) │
│ user_id (FK)│                              │ title           │
│ expires_at  │                              │ status          │
│ last_active │                              │ priority        │
└─────────────┘                              │ parent_id (FK→self)
                                             │ assigned_to     │
                                             │ position        │
                                             └────────┬────────┘
                                                      │
                                             ┌────────┴────────┐
                                             │ agent_sessions  │
                                             ├─────────────────┤
                                             │ id (PK)         │
                                             │ task_id (FK)    │
                                             │ agent_type      │
                                             │ status          │
                                             │ started_at      │
                                             │ finished_at     │
                                             │ (app-level:     │
                                             │  1 active/task) │
                                             └────────┬────────┘
                                                      │
                                             ┌────────┴─────────────┐
                                             │ agent_session_outputs│
                                             ├──────────────────────┤
                                             │ id (PK, auto)       │
                                             │ session_id (FK)     │
                                             │ sequence            │
                                             │ content             │
                                             │ (UQ: session+seq)   │
                                             └──────────────────────┘
```

### Status Enums

- **TaskStatus**: `backlog` → `todo` → `in_progress` → `in_review` → `done`
- **TaskPriority**: `low`, `medium`, `high`, `urgent`
- **AgentSessionStatus**: `pending` → `running` → `completed` | `failed` | `cancelled`
- **AgentType**: `claude_code`, `gemini_cli`

## Error Handling

```rust
AppError (thiserror)
├── NotFound(String)       → 404
├── Validation(String)     → 400
├── Conflict(String)       → 409
├── Unauthorized           → 401
├── Forbidden(String)      → 403
├── InvalidCredentials     → 401
├── Internal(String)       → 500
├── Database(sqlx::Error)  → 500 (UNIQUE/FK violations → 409)
├── Git(git2::Error)       → mapped by error code
└── Anyhow(anyhow::Error)  → 500
```

Response format: `{ "error": "message" }`

## API Client Generation

```
Backend                           Frontend
utoipa annotations                orval (orval.config.ts)
       │                                │
       ▼                                │
  openapi.json  ──────────────────────►│
                                        ▼
                                  src/api/generated/
                                  ├── endpoints/   (TanStack Query hooks)
                                  └── model/       (TypeScript 型定義)
```

Workflow: `task api:generate` で OpenAPI spec → TypeScript クライアント同期。

## Key Middleware

| Layer | Purpose | Config |
|-------|---------|--------|
| **Auth** | Cookie → Session → AuthUser extractor | `GANTRY_SESSION_DURATION_HOURS` |
| **CORS** | `AllowOrigin::exact(origin)` in production | `GANTRY_CORS_ORIGIN` |
| **Rate Limit (login)** | 5 req / 15 min per IP | tower_governor |
| **Rate Limit (register)** | 3 req / hour per IP | tower_governor |
| **Rate Limit (general API)** | ~1 req/s, 60 burst per IP | tower_governor |
| **Tracing** | HTTP request/response logging | `RUST_LOG` env |

## Development Commands

```bash
task backend:run       # Backend server
task frontend:dev      # Vite dev server
task backend:test      # cargo test
task frontend:test     # vitest
task api:generate      # OpenAPI → orval TypeScript client
task check             # clippy + biome lint + format
task fmt               # cargo fmt + biome format
```
