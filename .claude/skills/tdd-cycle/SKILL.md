---
name: tdd-cycle
description: TDD サイクルを実行してください。
user-invocable: true
---

# TDD Cycle

## Overview

テスト駆動開発 (Test-Driven Development) のサイクルを体系化したワークフロー。
テストを先に書き、失敗を確認してからコミットし、その後に実装を進める。

## TDD 7-Step Cycle

### Step 1: テスト作成

期待される入出力に基づいてテストを書く。実装コードは書かない。

- **Backend (Rust)**: `#[test]` / `#[tokio::test]` を使用
- **Frontend (TypeScript)**: Vitest で `describe` / `it` を使用
- 統合テストは `backend/tests/` に配置
- ユニットテストは対象モジュール内の `#[cfg(test)] mod tests` に配置

### Step 2: テスト実行 (失敗確認)

テストを実行し、**失敗すること**を確認する。

```bash
# Backend
task backend:test

# Frontend
task frontend:test

# 特定のテストのみ
cargo test -p gantry-board --lib <module>::tests::<test_name>
cargo test -p gantry-board --test <test_file> <test_name>
```

### Step 3: テストコミット

テストが正しく失敗することを確認したらコミット。

```
test: ✅ add <target> tests
```

- テストファイルのみをステージする
- 実装コードは含めない
- コンパイルエラーの場合は最小限のスタブ (空の構造体、`todo!()` 等) で通す

### Step 4: 実装

テストをパスさせるための最小限の実装を書く。

- テストは変更しない
- 実装コードのみを修正する

### Step 5: テスト再実行 (成功確認)

```bash
task backend:test   # または task frontend:test
```

全テストがパスするまで Step 4 を繰り返す。

### Step 6: リファクタリング (任意)

テストがパスした状態を維持しつつ、コードを整理する。

- テストは変更しない
- `task check` で品質確認

### Step 7: 実装コミット

```
feat: ✨ implement <feature>
fix: 🐛 fix <bug>
refactor: ♻️ extract <logic>
```

## Commit Pattern

```
test: ✅ add task creation tests             ← テストのみ (失敗状態)
feat: ✨ implement task creation              ← 実装 (テストパス)
refactor: ♻️ extract task validation logic   ← リファクタ (テスト維持)
```

- テストコミットと実装コミットは**必ず分離**する
- 実装フェーズでテストを変更しない
- 詳細は `commit-rules` スキルを参照

## Test Commands

| Layer | Command | Description |
|-------|---------|-------------|
| Backend (all) | `task backend:test` | cargo test (nextest) |
| Backend (unit) | `task backend:test:unit` | ユニットテストのみ (高速) |
| Frontend | `task frontend:test` | Vitest |
| Full check | `task check` | lint + build + test |

## Tips

- テストは**具体的な入出力**を検証する (振る舞いテスト)
- モックは最小限に。統合テストを優先する
- テスト名は `test_<action>_<condition>_<expected>` 形式
  - 例: `test_create_task_with_invalid_title_returns_validation_error`
- API テストでは `axum_test::TestServer` を使用
- DB テストでは `setup_test_db()` ヘルパーを使用
