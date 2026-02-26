# Operational Runbooks

Gantry Board の運用で遭遇する代表的なシナリオとその対処手順。

## Quick Reference

> **Note:** The URLs below use `localhost:3000` (native dev). When running via Docker Compose, the backend is on `localhost:3001`.

```bash
# Health check
curl -s http://localhost:3000/health | jq .
curl -s http://localhost:3000/health/live | jq .
curl -s http://localhost:3000/health/ready | jq .

# Prometheus metrics
curl -s http://localhost:3000/metrics

# Database integrity check
sqlite3 ./data/gantry_board.db "PRAGMA integrity_check;"

# WAL checkpoint (flush WAL to main DB)
sqlite3 ./data/gantry_board.db "PRAGMA wal_checkpoint(TRUNCATE);"

# Manual backup
sqlite3 ./data/gantry_board.db "VACUUM INTO './data/backups/manual_$(date +%Y%m%d_%H%M%S).db';"

# Disk usage
df -h /
du -sh ./data/gantry_board.db ./data/gantry_board.db-wal ./data/backups/

# Active agent sessions (via API — requires task_id)
curl -s http://localhost:3000/api/tasks/<task-id>/sessions | jq '.[].status'

# Worktree list
curl -s http://localhost:3000/api/worktrees | jq '.[].name'
```

---

## 1. Orphaned Agent Sessions

### Symptoms

- カンバンボード上でタスクが「Running」のまま長時間動かない
- サーバ再起動後に前回のセッションが残っている
- `GET /api/tasks/{task_id}/sessions` で `status: "running"` のセッションが存在するが、対応するプロセスが無い

### Diagnosis

```bash
# 実行中のセッション一覧
curl -s http://localhost:3000/api/tasks/<task-id>/sessions | jq '.[] | select(.status == "running")'

# 対応する PID が生きているか確認
ps -p <PID> -o pid,comm,etime 2>/dev/null || echo "Process not found"
```

### Resolution

サーバ起動時に自動リカバリが動作します（Issue #281 で実装済み）:

1. `status = "running"` のセッションを検出
2. `status = "failed"` に更新
3. 関連する worktree をベストエフォートでクリーンアップ

手動でリカバリする場合:

```bash
# サーバを再起動すると自動リカバリが実行される
systemctl restart gantry-board

# もしくは API 経由でセッションを停止
curl -X POST http://localhost:3000/api/tasks/<task-id>/sessions/<session-id>/stop
```

### Prevention

- サーバの graceful shutdown を行う（`SIGTERM` → 15 秒猶予 → `SIGKILL`）
- Docker Compose 使用時は `stop_grace_period: 15s` を設定
- プロセス監視ツール (systemd, supervisord) で自動再起動を設定

---

## 2. Database Corruption

### Symptoms

- ログに SQLite エラー: `database disk image is malformed`
- API レスポンスが 500 エラーを返す
- `/health/ready` が `ready: false` を返す

### Diagnosis

```bash
# Integrity check
sqlite3 ./data/gantry_board.db "PRAGMA integrity_check;"
# 正常なら "ok" が返る。異常なら詳細なエラーが表示される

# WAL ファイルの状態確認
ls -la ./data/gantry_board.db*

# ジャーナルモードの確認
sqlite3 ./data/gantry_board.db "PRAGMA journal_mode;"
# "wal" が返るべき
```

### Resolution

1. **軽度の破損** — WAL チェックポイントで回復を試みる:

```bash
sqlite3 ./data/gantry_board.db "PRAGMA wal_checkpoint(TRUNCATE);"
sqlite3 ./data/gantry_board.db "PRAGMA integrity_check;"
```

2. **重度の破損** — バックアップから復元する:

```bash
# 1. サーバを停止
systemctl stop gantry-board

# 2. 破損した DB を退避
mv ./data/gantry_board.db ./data/gantry_board.db.corrupted
rm -f ./data/gantry_board.db-wal ./data/gantry_board.db-shm

# 3. 最新のバックアップから復元
ls -lt ./data/backups/  # 最新のファイルを確認
cp ./data/backups/gantry_board_YYYYMMDD_HHMMSS.db ./data/gantry_board.db

# 4. サーバを再起動（マイグレーションが自動実行される）
systemctl start gantry-board
```

詳細な復元手順は [docs/BACKUP.md](./BACKUP.md) を参照。

### Prevention

- 自動バックアップを有効にする: `GANTRY_BACKUP_ENABLED=true` (default)
- バックアップ間隔を短くする: `GANTRY_BACKUP_INTERVAL_SECS=43200` (12h)
- WAL チェックポイントはセッションクリーンアップと同じ間隔で自動実行される
- UPS や安定した電源を使用し、突然の電源断を防ぐ

---

## 3. Disk Space Exhaustion

### Symptoms

- バックアップが失敗する: `VACUUM INTO` がエラーを返す
- ログに書き込みエラーが出る
- WAL ファイルが肥大化する

### Diagnosis

```bash
# ディスク全体の使用量
df -h /

# Gantry Board 関連ファイルのサイズ
du -sh ./data/gantry_board.db
du -sh ./data/gantry_board.db-wal
du -sh ./data/backups/
du -sh ./data/backups/* | sort -h

# Worktree のサイズ（git worktree 使用時）
du -sh .claude/worktrees/*/
```

### Resolution

1. **古いバックアップの削除**:

```bash
# 手動で古いバックアップを削除（最新3つを残す）
ls -t ./data/backups/gantry_board_*.db | tail -n +4 | xargs rm -f
```

2. **WAL ファイルの縮小**:

```bash
sqlite3 ./data/gantry_board.db "PRAGMA wal_checkpoint(TRUNCATE);"
```

3. **DB の VACUUM** (空き領域の回収):

```bash
# サーバ停止中に実行を推奨
sqlite3 ./data/gantry_board.db "VACUUM;"
```

4. **不要な worktree の削除**:

```bash
# API 経由で不要な worktree を削除
curl -X DELETE http://localhost:3000/api/worktrees/<name>
```

5. **古いエージェント出力のクリーンアップ**:

出力は `GANTRY_OUTPUT_RETENTION_DAYS` (default: 30) で自動削除される。緊急時は値を小さくしてサーバを再起動する。

### Prevention

- `GANTRY_BACKUP_RETENTION_COUNT` を適切に設定する (default: 7)
- `GANTRY_OUTPUT_RETENTION_DAYS` を適切に設定する (default: 30)
- ディスク使用率のモニタリングアラートを設定する (80% 以上で警告)
- `/metrics` の `gantry_db_pool_connections` を監視する

---

## 4. Docker Socket Failure

### Symptoms

- プレビュー (Preview) の作成が失敗する
- ログに Docker 接続エラー: `Cannot connect to the Docker daemon`
- コンテナ操作がタイムアウトする

### Diagnosis

```bash
# Docker デーモンの状態確認
systemctl status docker
docker ps

# ソケットファイルの存在と権限確認
ls -la /var/run/docker.sock

# Gantry Board の設定確認
echo $GANTRY_DOCKER_HOST
# default: unix:///var/run/docker.sock

# Gantry Board プロセスのユーザーが docker グループに属しているか
id $(whoami)
groups $(whoami)
```

### Resolution

1. **Docker デーモンの再起動**:

```bash
systemctl restart docker
```

2. **ソケット権限の修正**:

```bash
# Gantry Board 実行ユーザーを docker グループに追加
sudo usermod -aG docker $(whoami)
# ※ セッションの再ログインが必要
```

3. **カスタムソケットパスの設定**:

```bash
export GANTRY_DOCKER_HOST="unix:///path/to/custom/docker.sock"
```

4. **TCP 経由の Docker 接続** (リモートホスト):

```bash
# TLS で保護されたリモート Docker デーモン (ポート 2376) を使用
export GANTRY_DOCKER_HOST="tcp://docker-host:2376"
```

> **Warning**: ポート 2375 のような平文・未認証の Docker TCP エンドポイントは、テスト用途以外では使用しないでください。本番環境では TLS + クライアント証明書認証を有効にしてください。

### Prevention

- Docker デーモンの自動再起動を設定: `systemctl enable docker`
- Docker ヘルスチェックを監視する
- `GANTRY_DOCKER_HOST` を明示的に設定ファイルに記載する

---

## 5. Memory Pressure / OOM

### Symptoms

- プロセスが突然終了する
- `dmesg` に OOM Killer の記録がある
- レスポンスが極端に遅くなる

### Diagnosis

```bash
# OOM Killer の履歴
dmesg | grep -i "oom\|killed process"

# プロセスのメモリ使用量
ps aux | grep gantry

# DB プール接続数 (Prometheus メトリクス)
curl -s http://localhost:3000/metrics | grep gantry_db_pool

# システム全体のメモリ使用量
free -h
```

### Resolution

1. **DB 接続プールの縮小**:

```bash
export GANTRY_MAX_DB_CONNECTIONS=10  # default: 20
```

2. **リアルタイム接続数の制限**:

```bash
export GANTRY_MAX_REALTIME_CONNECTIONS=50  # default: 100
```

3. **SSE ブロードキャストチャネルの縮小**:

```bash
export GANTRY_SSE_BROADCAST_CAPACITY=1024  # default: 4096
```

4. **スワップの追加** (緊急対応):

```bash
sudo fallocate -l 2G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

### Prevention

- `GANTRY_MAX_DB_CONNECTIONS` と `GANTRY_MAX_REALTIME_CONNECTIONS` を利用パターンに合わせて調整
- Docker Compose でメモリ制限を設定: `mem_limit: 1g`
- Prometheus + Alertmanager でメモリ使用率アラートを設定
- `/metrics` エンドポイントで `gantry_db_pool_connections` を定期監視

---

## 6. GitHub Sync Failures

### Symptoms

- GitHub Issues がカンバンボードに反映されない
- Webhook deliveries が失敗している (GitHub Settings > Webhooks で確認)
- ログに `github sync` 関連のエラーが出る

### Diagnosis

```bash
# Gantry Board のログで同期エラーを確認
journalctl -u gantry-board | grep -i "github\|sync"

# GitHub token の有効性テスト
curl -H "Authorization: token $GANTRY_GITHUB_TOKEN" https://api.github.com/rate_limit

# Webhook secret が設定されているか確認
echo "GANTRY_GITHUB_TOKEN=${GANTRY_GITHUB_TOKEN:+SET}"
echo "GANTRY_GITHUB_WEBHOOK_SECRET=${GANTRY_GITHUB_WEBHOOK_SECRET:+SET}"

# GitHub API rate limit 確認
curl -s -H "Authorization: token $GANTRY_GITHUB_TOKEN" \
  https://api.github.com/rate_limit | jq '.rate'
```

### Resolution

1. **Token の再発行** (token 期限切れの場合):
   - GitHub Settings > Developer settings > Personal access tokens で新しいトークンを発行
   - `GANTRY_GITHUB_TOKEN` を更新してサーバを再起動

2. **Rate limit 超過の場合**:
   - 同期間隔を延ばす: `GANTRY_GITHUB_SYNC_INTERVAL_SECS=600` (10 分)
   - rate limit リセット時刻まで待つ

3. **Webhook 配信エラーの場合**:
   - GitHub リポジトリの Settings > Webhooks で配信履歴を確認
   - Webhook URL がサーバに到達可能か確認
   - `GANTRY_GITHUB_WEBHOOK_SECRET` が正しいか確認

4. **手動同期のトリガー**:

```bash
# サーバを再起動すると同期が再開される
systemctl restart gantry-board
```

### Prevention

- `GANTRY_GITHUB_WEBHOOK_SECRET` を必ず設定する (署名検証を有効化)
- Token に必要最小限のスコープのみ付与する
- `GANTRY_GITHUB_SYNC_INTERVAL_SECS` を API rate limit に合わせて調整 (default: 300s)
- token の有効期限を監視し、期限前にローテーションする

---

## 7. Backup and Restore

### Symptoms

- データ損失が発生し、バックアップからの復元が必要
- マイグレーション失敗後のロールバックが必要

### Diagnosis

```bash
# 利用可能なバックアップの一覧
ls -lht ./data/backups/

# バックアップの整合性検証
sqlite3 ./data/backups/gantry_board_YYYYMMDD_HHMMSS.db "PRAGMA integrity_check;"

# バックアップ内のテーブル一覧
sqlite3 ./data/backups/gantry_board_YYYYMMDD_HHMMSS.db ".tables"
```

### Resolution

完全な復元手順:

```bash
# 1. サーバを停止
systemctl stop gantry-board

# 2. 現在の DB を退避（ロールバック用）
cp ./data/gantry_board.db ./data/gantry_board.db.pre-restore
cp ./data/gantry_board.db-wal ./data/gantry_board.db-wal.pre-restore 2>/dev/null

# 3. バックアップから復元
cp ./data/backups/gantry_board_YYYYMMDD_HHMMSS.db ./data/gantry_board.db

# 4. WAL/SHM ファイルを削除（新しい DB に古い WAL を適用させない）
rm -f ./data/gantry_board.db-wal ./data/gantry_board.db-shm

# 5. サーバを再起動（マイグレーションが自動実行される）
systemctl start gantry-board

# 6. 動作確認
curl -s http://localhost:3000/health | jq .
```

詳細な手順は [docs/BACKUP.md](./BACKUP.md) を参照。

### Prevention

- `GANTRY_BACKUP_ENABLED=true` を維持する
- `GANTRY_BACKUP_RETENTION_COUNT` を十分な値にする (default: 7)
- 定期的にバックアップの整合性を確認する
- 復元手順を定期的にテストする

---

## 8. Configuration Reference

すべての環境変数は `GANTRY_` プレフィックスで設定する。`config.toml` ファイルでも設定可能（環境変数が優先）。

### Server

| Variable | Default | Description |
|----------|---------|-------------|
| `GANTRY_BIND_ADDR` | `0.0.0.0:3000` | バインドアドレス |
| `GANTRY_LOG_FORMAT` | `pretty` | ログフォーマット (`pretty` or `json`) |
| `GANTRY_REQUEST_TIMEOUT_SECS` | `60` | HTTP リクエストタイムアウト (秒) |
| `GANTRY_ALLOWED_HOSTS` | *(empty)* | 許可する Host ヘッダー値 (空=検証なし) |

### Database

| Variable | Default | Description |
|----------|---------|-------------|
| `GANTRY_DATABASE_URL` | `sqlite:./data/gantry_board.db?mode=rwc` | SQLite 接続 URL |
| `GANTRY_MAX_DB_CONNECTIONS` | `20` | DB コネクションプール最大サイズ |

### Authentication

| Variable | Default | Description |
|----------|---------|-------------|
| `GANTRY_SESSION_DURATION_HOURS` | `168` (1 week) | セッション有効期間 (時間) |
| `GANTRY_COOKIE_SECURE` | `true` | Secure cookie (HTTPS のみ) |
| `GANTRY_CORS_ORIGIN` | *(unset)* | 許可する CORS オリジン (**本番環境では必須**) |
| `GANTRY_AUTH_DISABLED` | `false` | 認証無効化 (debug ビルドのみ) |
| `GANTRY_SESSION_CLEANUP_INTERVAL_SECS` | `3600` (1h) | セッションクリーンアップ間隔 (秒) |

### Rate Limiting

| Variable | Default | Description |
|----------|---------|-------------|
| `GANTRY_REGISTER_RATE_LIMIT_PER_SECOND` | `1200` | 登録 rate limit 補充間隔 (秒) |
| `GANTRY_REGISTER_RATE_LIMIT_BURST` | `3` | 登録 rate limit バーストサイズ |
| `GANTRY_LOGIN_RATE_LIMIT_PER_SECOND` | `180` | ログイン rate limit 補充間隔 (秒) |
| `GANTRY_LOGIN_RATE_LIMIT_BURST` | `5` | ログイン rate limit バーストサイズ |
| `GANTRY_GENERAL_RATE_LIMIT_PER_SECOND` | `1` | 一般 API rate limit 補充間隔 (秒) |
| `GANTRY_GENERAL_RATE_LIMIT_BURST` | `60` | 一般 API rate limit バーストサイズ |

### Real-time

| Variable | Default | Description |
|----------|---------|-------------|
| `GANTRY_SSE_BROADCAST_CAPACITY` | `4096` | SSE ブロードキャストチャネル容量 |
| `GANTRY_MAX_REALTIME_CONNECTIONS` | `100` | SSE + WebSocket 最大同時接続数 |

### GitHub Integration

| Variable | Default | Description |
|----------|---------|-------------|
| `GANTRY_GITHUB_TOKEN` | *(unset)* | GitHub Personal Access Token |
| `GANTRY_GITHUB_SYNC_INTERVAL_SECS` | `300` (5 min) | GitHub 同期間隔 (秒, 最小: 60) |
| `GANTRY_GITHUB_WEBHOOK_SECRET` | *(unset)* | Webhook 署名検証シークレット |

### Docker / Preview

| Variable | Default | Description |
|----------|---------|-------------|
| `GANTRY_DOCKER_HOST` | `unix:///var/run/docker.sock` | Docker デーモンソケット |
| `GANTRY_PREVIEW_PORT_RANGE_START` | `8100` | プレビューポート範囲の開始 |
| `GANTRY_PREVIEW_PORT_RANGE_END` | `8199` | プレビューポート範囲の終了 |
| `GANTRY_PREVIEW_BASE_URL` | `http://localhost` | プレビューのベース URL |
| `GANTRY_REPOSITORY_PATH` | *(unset)* | Git リポジトリパス (worktree 管理用) |

### Backup

| Variable | Default | Description |
|----------|---------|-------------|
| `GANTRY_BACKUP_ENABLED` | `true` | 自動バックアップの有効/無効 |
| `GANTRY_BACKUP_DIR` | `./data/backups` | バックアップファイルの保存先 |
| `GANTRY_BACKUP_INTERVAL_SECS` | `86400` (24h) | バックアップ間隔 (秒) |
| `GANTRY_BACKUP_RETENTION_COUNT` | `7` | 保持するバックアップ数 |

### Agent Session Output

| Variable | Default | Description |
|----------|---------|-------------|
| `GANTRY_OUTPUT_RETENTION_DAYS` | `30` | エージェント出力の保持期間 (日) |

### Health Check Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /health` | 総合ヘルスチェック (DB 接続確認含む) |
| `GET /health/live` | Liveness probe (プロセスが生きているか) |
| `GET /health/ready` | Readiness probe (リクエスト受付可能か) |
| `GET /metrics` | Prometheus メトリクス |

### Production Checklist

- [ ] `GANTRY_CORS_ORIGIN` を明示的に設定する
- [ ] `GANTRY_COOKIE_SECURE=true` を確認する (HTTPS 環境)
- [ ] `GANTRY_ALLOWED_HOSTS` にサーバのホスト名を設定する
- [ ] `GANTRY_GITHUB_WEBHOOK_SECRET` を設定する (GitHub 連携使用時)
- [ ] `GANTRY_LOG_FORMAT=json` を設定する (構造化ログ)
- [ ] `GANTRY_BACKUP_ENABLED=true` を確認する
- [ ] `/health/ready` をロードバランサーの health check に使用する
- [ ] `/metrics` を Prometheus で scrape する
- [ ] ファイアウォールで `GANTRY_BIND_ADDR` のポートのみ開放する
