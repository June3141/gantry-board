#!/usr/bin/env bash
# テスト戦略・ディレクトリ構成の改善 Issue 一括作成スクリプト
# 使い方: gh auth login 済みの環境で実行
#   chmod +x docs/issues/create-test-improvement-issues.sh
#   ./docs/issues/create-test-improvement-issues.sh
set -euo pipefail

REPO="June3141/gantry-board"

echo "=== Issue 1/6: test_helpers モジュール昇格 ==="
gh issue create --repo "$REPO" \
  --title "refactor: 🧪 Promote test_helpers.rs to test_helpers/ module with shared fixtures" \
  --label "testing,refactoring,backend,high-priority,size:M" \
  --body "$(cat <<'EOF'
## 背景

`task_service/tests.rs` 内にローカル定義された `create_test_project`, `create_test_user`, `add_test_member` が、他のサービステスト追加時に再利用できない状態。テストヘルパーが3箇所に分散している：

| 場所 | 内容 |
|------|------|
| `backend/src/test_helpers.rs` | DB セットアップのみ (22行) |
| `backend/tests/common/mod.rs` | 結合テスト用ヘルパー (`create_test_server` 等) |
| `backend/src/services/task_service/tests.rs` | ローカル fixture 定義 |

## 改善内容

`test_helpers.rs` を `test_helpers/` モジュールに昇格し、ドメインオブジェクト生成ヘルパーを集約する。

```
backend/src/test_helpers/
├── mod.rs           — pub mod db; pub mod fixtures;
├── db.rs            — setup_test_db()
└── fixtures.rs      — create_test_project, create_test_user, add_test_member
```

## 期待効果

- テストコードの DRY 化
- 新しいサービスユニットテスト追加の障壁低下
- テストデータ生成の一貫性確保

## 参考

t-wada「テストコードもプロダクションコードと同じ品質で管理すべき」
EOF
)"

echo "=== Issue 2/6: renderWithProviders 統一 ==="
gh issue create --repo "$REPO" \
  --title "refactor: 🧪 Unify renderWithProviders into test/helpers/" \
  --label "testing,refactoring,frontend,high-priority,size:S" \
  --body "$(cat <<'EOF'
## 背景

`renderWithProviders` (QueryClient + Router のラッパー) が複数のテストファイルで重複定義されている：

- `taskDetailModalSetup.ts` — `renderWithMocks()`
- `KanbanBoard.test.tsx` — ローカル定義
- その他のコンポーネントテストでも類似パターンあり

## 改善内容

共通の render ヘルパーを `test/helpers/` に集約する。

```
frontend/src/test/
├── setup.ts
├── mocks/
│   ├── factories.ts
│   ├── handlers.ts
│   └── server.ts
└── helpers/
    └── renderWithProviders.ts  — QueryClient + Router ラッパー
```

## 期待効果

- テストセットアップの重複排除
- 新しいコンポーネントテスト追加時のボイラープレート削減
- テスト基盤の一貫性向上
EOF
)"

echo "=== Issue 3/6: Factory パターン統一 ==="
gh issue create --repo "$REPO" \
  --title "refactor: 🧪 Unify test data factories across all frontend tests" \
  --label "testing,refactoring,frontend,high-priority,size:M" \
  --body "$(cat <<'EOF'
## 背景

テストデータの生成が複数箇所に分散している：

| 場所 | 内容 |
|------|------|
| `test/mocks/factories.ts` | `buildTask`, `buildMember`, `buildComment` (正式な factory) |
| `KanbanBoard.test.tsx` | `createMockTask()` (ローカル定義) |
| `taskDetailModalSetup.ts` | `mockTask` (ローカル定数) |
| その他テストファイル | インラインでテストデータを定義 |

## 改善内容

1. `factories.ts` の既存 builder を全テストで一貫して使用
2. ローカル定義のファクトリ関数・定数を除去
3. 必要に応じて `factories.ts` に新しい builder を追加

## 期待効果

- テストデータの一貫性確保
- API モデル変更時の修正箇所が1箇所に
- テストの可読性向上 (builder パターンで意図が明確)

## 参考

t-wada「テストフィクスチャの管理はテスト品質の根幹」
EOF
)"

echo "=== Issue 4/6: authorization_service ユニットテスト ==="
gh issue create --repo "$REPO" \
  --title "test: ✅ Add unit tests for authorization_service" \
  --label "testing,backend,size:M" \
  --body "$(cat <<'EOF'
## 背景

現在 `authorization_service` のテストは結合テスト (`backend/tests/authorization/`) のみでカバーされている。権限チェックロジックはビジネスクリティカルであり、結合テストでは:

- フィードバックが遅い (DB セットアップのオーバーヘッド)
- 境界条件の網羅的テストがやりにくい
- デバッグ時の原因特定が困難

## 改善内容

`authorization_service` に対してサービス層のユニットテストを追加する。

### テスト対象の例

- owner のみが許可される操作 (プロジェクト削除等)
- member が許可される操作
- 非メンバーが拒否される操作
- 複数ロールの境界条件

### 構造

```
backend/src/services/authorization_service/
├── mod.rs          — ビジネスロジック
└── tests.rs        — ユニットテスト
```

または既存ファイル内の `#[cfg(test)] mod tests` でも可。

## 期待効果

- 権限バグの早期検出
- テスト実行速度改善 (Small テストの充実)
- テストピラミッドの健全化 (バックエンド Small テストの拡充)

## 追加候補

同様のアプローチで以下のサービスにもユニットテスト追加を検討:
- `agent_session_service` (ライフサイクル管理)
- `invitation_service` (トークン生成・期限管理)
- `github_sync_service` (同期ロジック)
EOF
)"

echo "=== Issue 5/6: コンポーネントテストの MSW 移行 ==="
gh issue create --repo "$REPO" \
  --title "refactor: 🧪 Migrate component tests from vi.mock to MSW-based approach" \
  --label "testing,refactoring,frontend,size:L" \
  --body "$(cat <<'EOF'
## 背景

現在のコンポーネントテストには2つのモック方式が混在している：

| 方式 | 利用箇所 | 特徴 |
|------|---------|------|
| MSW (Mock Service Worker) | `test/mocks/handlers.ts` | ネットワーク層でインターセプト (本物に近い) |
| vi.mock | `taskDetailModalSetup.ts` 等 | API hook を直接モック (実装詳細に依存) |

`taskDetailModalSetup.ts` の `setupMocks()` は13個もの API hook を個別にモックしており、実装の詳細への過度な結合が発生している。API の内部構造 (hook名、戻り値型) が変わるたびにテストが壊れる。

## 改善方針

| 方針 | 対象 | 方法 |
|------|------|------|
| API呼び出し | 全コンポーネントテスト | **MSW** |
| Router | ナビゲーションテスト | vi.mock (最小限) |
| Store | ストア単体テスト | 直接操作 (現状維持) |

## 段階的移行

1. **`TaskDetailModal` テスト** — 最大の vi.mock 依存。MSW handler を追加して移行
2. **`KanbanBoard` テスト** — MSW 化で実装結合を緩和
3. **その他のコンポーネント** — 順次対応

## 期待効果

- リファクタリング耐性の向上 (内部 hook 名の変更に強くなる)
- テストの信頼性向上 (実際の HTTP リクエスト/レスポンスフローを検証)
- テストコードの保守性向上

## 参考

t-wada「テストダブルは最小限に。できるだけ本物に近いものを使え」
EOF
)"

echo "=== Issue 6/6: routes.rs 分離 ==="
gh issue create --repo "$REPO" \
  --title "refactor: 📦 Extract route definitions from lib.rs into routes.rs" \
  --label "refactoring,backend,size:S" \
  --body "$(cat <<'EOF'
## 背景

現在 `backend/src/lib.rs` にルーティング定義 (~440行) が集中している。現時点では許容範囲だが、エンドポイントの追加に伴い肥大化が予想される。

## 改善内容

ルーティング定義を `backend/src/routes.rs` に分離する。

### Before

```rust
// lib.rs (~440行)
pub fn create_app(...) -> Router {
    Router::new()
        .route("/api/tasks", get(handlers::tasks::list))
        // ... 数十行のルート定義
}
```

### After

```rust
// routes.rs
pub fn api_routes() -> Router<AppState> { ... }

// lib.rs (大幅に短縮)
pub fn create_app(...) -> Router {
    routes::api_routes().with_state(state)
}
```

## 判断基準

- lib.rs が 500行を超えた時点で着手を検討
- 現時点 (~440行) では優先度低

## 期待効果

- `lib.rs` の責務を app 初期化に限定
- ルート定義の見通し改善
- ルートのグループ化 (public/authenticated/admin) が容易に
EOF
)"

echo ""
echo "✅ 全6件の Issue を作成しました"
