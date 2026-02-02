# Gantry Board - 導入ツール・プラグイン・スキル一覧

このドキュメントは、開発に使用するプラグイン・スキル・サブエージェントとその参照元を記録する。

---

## 公式プラグイン (Anthropic)

インストール: `/plugin install {name}@claude-plugins-official`

| プラグイン | 用途 | URL |
|-----------|------|-----|
| rust-analyzer-lsp | Rust コード解析・補完・診断 | https://github.com/anthropics/claude-plugins-official/tree/main/plugins/rust-analyzer-lsp |
| typescript-lsp | TypeScript 支援 | https://github.com/anthropics/claude-plugins-official/tree/main/plugins/typescript-lsp |
| code-review | 5 並列 Sonnet によるコードレビュー | https://github.com/anthropics/claude-plugins-official/tree/main/plugins/code-review |
| pr-review-toolkit | PR レビュー自動化 | https://github.com/anthropics/claude-plugins-official/tree/main/plugins/pr-review-toolkit |
| feature-dev | ガイド付き機能開発 | https://github.com/anthropics/claude-plugins-official/tree/main/plugins/feature-dev |
| code-simplifier | リファクタリング支援 | https://github.com/anthropics/claude-plugins-official/tree/main/plugins/code-simplifier |
| commit-commands | コミットコマンド強化 | https://github.com/anthropics/claude-plugins-official/tree/main/plugins/commit-commands |
| security-guidance | セキュリティガイダンス | https://github.com/anthropics/claude-plugins-official/tree/main/plugins/security-guidance |
| hookify | hooks 作成支援 | https://github.com/anthropics/claude-plugins-official/tree/main/plugins/hookify |
| claude-md-management | CLAUDE.md 管理 | https://github.com/anthropics/claude-plugins-official/tree/main/plugins/claude-md-management |
| frontend-design | 本番品質 UI 生成 | https://github.com/anthropics/claude-plugins-official/tree/main/plugins/frontend-design |

公式プラグインリポジトリ: https://github.com/anthropics/claude-plugins-official

### product-management プラグイン

Anthropic 公式 (claude.com 経由)。コマンド: `/write-spec`, `/roadmap-update`, `/synthesize-research`, `/competitive-brief`, `/metrics-review`

URL: https://claude.com/plugins/product-management

---

## 外部プラグイン

| プラグイン | 用途 | URL |
|-----------|------|-----|
| context7 | ライブラリドキュメント参照 | https://github.com/anthropics/claude-plugins-official/tree/main/external_plugins/context7 |
| playwright | E2E テスト | https://github.com/anthropics/claude-plugins-official/tree/main/external_plugins/playwright |
| github | GitHub 連携強化 | https://github.com/anthropics/claude-plugins-official/tree/main/external_plugins/github |

---

## Vercel Agent Skills (MIT)

インストール: `npx skills add vercel-labs/agent-skills`

| スキル | 用途 | URL |
|-------|------|-----|
| web-design-guidelines | 100+ ルール A11y・パフォーマンス・UX 監査 | https://github.com/vercel-labs/agent-skills |
| react-best-practices | 40+ ルール React パフォーマンス最適化 | https://github.com/vercel-labs/agent-skills |
| composition-patterns | コンポーネント設計パターン | https://github.com/vercel-labs/agent-skills |

---

## 公式スキル (Anthropic Skills リポジトリ)

インストール: `/plugin marketplace add anthropics/skills` → `/plugin install {name}@anthropic-agent-skills`

| スキル | 用途 | URL |
|-------|------|-----|
| webapp-testing | Playwright による Web アプリテスト | https://github.com/anthropics/skills/tree/main/skills/webapp-testing |
| skill-creator | カスタムスキル作成ガイド | https://github.com/anthropics/skills/tree/main/skills/skill-creator |

公式スキルリポジトリ: https://github.com/anthropics/skills

---

## カスタムサブエージェント (.claude/agents/)

| エージェント | 用途 | 参照元 |
|-------------|------|--------|
| code-reviewer.md | コード品質チェック | https://github.com/VoltAgent/awesome-claude-code-subagents |
| architect-reviewer.md | アーキテクチャレビュー | https://github.com/VoltAgent/awesome-claude-code-subagents |
| qa-expert.md | テスト自動化支援 | https://github.com/VoltAgent/awesome-claude-code-subagents |
| error-detective.md | エラー分析・解決 | https://github.com/VoltAgent/awesome-claude-code-subagents |

---

## カスタムスキル参考元

| スキル | 用途 | 参照元 |
|-------|------|--------|
| differential-review | セキュリティ重視の diff レビュー | https://github.com/VoltAgent/awesome-agent-skills (Trail of Bits) |
| static-analysis | CodeQL/Semgrep 静的解析 | https://github.com/VoltAgent/awesome-agent-skills (Trail of Bits) |

---

## 参考ワークフロー (直接導入ではなく設計参考)

| ツール | 参考にする点 | URL | ライセンス |
|-------|-------------|-----|-----------|
| CCPM | PRD→Epic→Task→Issue→Code ワークフロー、GitHub Issues + worktree 連携 | https://github.com/automazeio/ccpm | MIT |
| Vibe Kanban | Rust + TS アーキテクチャ、CLI サブプロセス方式、MCP 統合 | https://github.com/BloopAI/vibe-kanban | Apache-2.0 |
