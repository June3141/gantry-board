---
name: feature-dev
description: Feature 開発の標準ワークフローを実行してください。
user-invocable: true
---

# Feature Development Workflow

## Overview

Feature 開発の標準フロー。ブランチ作成から PR マージまでの全工程を体系化する。
各ステップは既存スキルと連携している。

## Step 1: ブランチ作成

`branch-strategy` スキルに従ってブランチを作成する。

```bash
git checkout develop
git pull origin develop
git checkout -b feat/<issue-id>-<slug>
```

- Issue ID を含める (例: `feat/42-kanban-board`)
- 機能単位でブランチを分ける

## Step 2: TDD サイクル

`tdd-cycle` スキルに従ってテスト駆動で実装する。

1. テストを書く
2. テスト実行 → 失敗確認
3. `test: ✅ add <target> tests` でコミット
4. 実装を書く
5. テスト実行 → 成功確認
6. `feat: ✨ implement <feature>` でコミット

## Step 3: API 変更時の追加手順

OpenAPI アノテーション (`utoipa`) を変更した場合:

```bash
task api:generate    # TypeScript クライアント再生成
```

再生成後にコミット:

```
chore: 🔧 regenerate API client for <feature> endpoint
```

## Step 4: 品質確認

`quality-checks` スキルのコマンドで品質を確認する。

```bash
task check           # lint + build + test (L3 フックで自動実行)
task backend:test    # Backend テスト
task frontend:test   # Frontend テスト
```

## Step 5: コミット

`commit-rules` スキルに従ってコミットする。

- 1 コミット = 1 関心事
- Max 10 files / 300 lines (テスト・自動生成除く)
- テストと実装は分離

## Step 6: PR 作成

```bash
git push -u origin feat/<issue-id>-<slug>
gh pr create --base develop --title "<type>: <emoji> <subject>"
```

PR body には以下を含める:
- **Summary**: 変更内容 1-3 文
- **Test plan**: 検証手順チェックリスト
- `Closes #<issue>` で issue を参照

## Skill Dependencies

```
feature-dev
├── branch-strategy   … ブランチ命名・マージルール
├── tdd-cycle         … TDD 7-step サイクル
├── quality-checks    … lint/test/build コマンド
└── commit-rules      … コミットメッセージ形式・PR テンプレート
```

## Checklist

Feature 完成時の確認項目:

- [ ] テストが全てパスする (`task test`)
- [ ] Lint 警告がゼロ (`task lint`)
- [ ] API 変更があれば orval 再生成済み
- [ ] コミットが `type: emoji subject` 形式
- [ ] テストコミットと実装コミットが分離されている
- [ ] PR が `Closes #<issue>` で issue を参照
