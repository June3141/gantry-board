# SQLite Backup and Restore

## Overview

Gantry Board automatically backs up its SQLite database on a configurable schedule. Backups use SQLite's `VACUUM INTO` command, which creates a consistent, standalone copy of the database even while the application is running.

## Configuration

Backup behavior is controlled via environment variables (or `config.toml`):

| Variable | Default | Description |
|----------|---------|-------------|
| `GANTRY_BACKUP_ENABLED` | `true` | Enable/disable automated backups |
| `GANTRY_BACKUP_DIR` | `./data/backups` | Directory for backup files |
| `GANTRY_BACKUP_INTERVAL_SECS` | `86400` (24h) | Interval between backups |
| `GANTRY_BACKUP_RETENTION_COUNT` | `7` | Number of backups to retain |

## Automated Backups

When `GANTRY_BACKUP_ENABLED=true` (the default), the application:

1. Spawns a background task that runs every `GANTRY_BACKUP_INTERVAL_SECS` seconds
2. Flushes the WAL with `PRAGMA wal_checkpoint(TRUNCATE)`
3. Creates a backup with `VACUUM INTO` at `<backup_dir>/gantry_board_YYYYMMDD_HHMMSS.db`
4. Rotates old backups, keeping only `GANTRY_BACKUP_RETENTION_COUNT` most recent files

## Manual Backup

To create a manual backup using the task runner:

```bash
task db:backup
```

Or using `sqlite3` directly:

```bash
sqlite3 ./data/gantry_board.db "VACUUM INTO './data/backups/manual_backup.db';"
```

## Restore from Backup

To restore from a backup:

1. Stop the Gantry Board application
2. Replace the database file with the backup:

```bash
# Stop the application first
cp ./data/gantry_board.db ./data/gantry_board.db.old  # keep current as safety net
cp ./data/backups/gantry_board_20260221_120000.db ./data/gantry_board.db
```

3. Remove any WAL and SHM files:

```bash
rm -f ./data/gantry_board.db-wal ./data/gantry_board.db-shm
```

4. Restart the application

## Backup File Format

- Backup files are named `gantry_board_YYYYMMDD_HHMMSS.db`
- Each backup is a standalone SQLite database file
- Backups include all data and schema but exclude the WAL/SHM files
- Backups can be opened independently with any SQLite client for inspection

## Monitoring

The application logs backup events at the following levels:

- `INFO`: Successful backup creation (includes file size and duration)
- `INFO`: Backup rotation (number of old files deleted)
- `WARN`: Rotation failures (non-fatal)
- `ERROR`: Backup creation failures
