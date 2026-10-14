#!/usr/bin/env python3
"""Exercise archive backups/restores and compare inner compression on Compose's db.

Requires a running `docker compose up -d db`, PostgreSQL client tools, xz and
zstd. Uses uniquely named fixture databases and an isolated HOME; removes only
the databases it creates. Does not start or stop Compose services.
"""

import argparse
import json
import os
from pathlib import Path
import statistics
import subprocess
import tempfile
import time
import uuid


ROOT = Path(__file__).resolve().parents[1]


def run(command, *, env=None, input=None, expected=0):
    result = subprocess.run(
        [str(arg) for arg in command],
        cwd=ROOT,
        env=env,
        input=input,
        text=True,
        capture_output=True,
        timeout=180,
    )
    if result.returncode != expected:
        # Avoid logging environment values or command arguments with credentials.
        raise RuntimeError(
            f"{Path(str(command[0])).name} exited {result.returncode}: "
            f"{result.stderr[-3000:]}"
        )
    return result.stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/release/pg_actions")
    parser.add_argument("--rows", type=int, default=100_000)
    parser.add_argument("--benchmark-runs", type=int, default=3)
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    if args.rows < 1 or args.benchmark_runs < 1:
        parser.error("rows and benchmark-runs must be positive")
    binary = args.binary.resolve()
    if not binary.is_file():
        parser.error("Build the binary first with cargo build --locked --release")

    # Capture Compose credentials without printing or storing them in artifacts.
    credentials = run([
        "docker", "compose", "exec", "-T", "db", "sh", "-c",
        'printf "%s\\n%s" "${POSTGRES_USER:-postgres}" "$POSTGRES_PASSWORD"',
    ])
    user, password = credentials.split("\n", 1)
    port = run(["docker", "compose", "port", "db", "5432"]).strip().splitlines()[0].rsplit(":", 1)[1]
    env = os.environ.copy()
    # Do not inherit app options that can redirect or change the test backups.
    for name in [
        "BACKUP_LOCATION", "RESTORE_LOCATION", "FORMAT", "NO_ARCHIVE", "JOBS",
        "COMPRESSION", "COMPRESSION_LEVEL", "COMPRESSION_THREADS",
        "PG_DUMP_COMPRESSION", "FILE_NAME", "OVERWRITE", "KEEP_TEMP",
        "PG_DUMP", "PG_RESTORE", "PSQL", "USE_JSON_LOGGING",
    ]:
        env.pop(name, None)
    env.update(
        PGHOST="127.0.0.1", PG_HOSTNAME="127.0.0.1",
        PGPORT=port, PG_PORT=port, PGUSER=user, PG_SUPERUSER=user,
        PGPASSWORD=password, PG_PASS=password,
    )
    databases = []
    prefix = "pg_actions_test_" + uuid.uuid4().hex[:10]
    source = prefix + "_source"

    def sql(database, statement):
        return run(["psql", "-X", "-v", "ON_ERROR_STOP=1", "-At", "-d", database], env=env, input=statement).strip()

    def fingerprint(database):
        return sql(database, "SELECT count(*)::text || ':' || md5(string_agg(md5(ROW(id, category, payload)::text), '' ORDER BY id)) FROM fixture;")

    with tempfile.TemporaryDirectory(prefix="pg_actions-integration-") as directory:
        workspace = Path(directory)
        home = workspace / "home"
        home.mkdir()
        env.update(HOME=str(home), LOG_FILE=str(workspace / "pg_actions.log"))

        def cli(*arguments, expected=0):
            return run([binary, *arguments], env=env, expected=expected)

        def restore(path, name, *, format="bsql"):
            target = prefix + "_" + name
            databases.append(target)
            cli("restore", target, "-i", path, "--file", source, "--format", format)
            assert fingerprint(target) == original, f"Restored data differs: {name}"
            print(f"PASS restore: {name}", flush=True)
            return target

        def extract(path, codec, destination):
            destination.mkdir()
            # Independent system decoder validates the entire compression stream.
            run([codec, "--test", path])
            run(["tar", "-xf", path, "-C", destination])
            return destination / (source + ".bsql")

        def dump_codec(path):
            listing = run(["pg_restore", "--list", path])
            for line in listing.splitlines():
                field, _, value = line.lstrip("; ").partition(":")
                if field == "Compression":
                    return value.strip()
            raise RuntimeError("pg_restore did not report the dump compression method")

        try:
            sql("postgres", f'CREATE DATABASE "{source}";')
            databases.append(source)
            sql(source, f"""
                CREATE TABLE fixture(id integer PRIMARY KEY, category integer, payload text NOT NULL);
                INSERT INTO fixture
                SELECT i, i % 31,
                    repeat('backup fixture ' || (i % 31)::text || ' ', 12)
                    || md5(i::text) || md5((i * 17)::text)
                FROM generate_series(1, {args.rows}) AS i;
                CREATE INDEX ON fixture(category);
                ANALYZE fixture;
            """)
            original = fingerprint(source)

            # all --no-archive still creates an archive, with the correct suffix.
            all_path = workspace / "all"
            cli("backup", "all", "--no-archive", "-j", "2", "-o", all_path)
            all_path = workspace / "all.tar.zst"
            assert all_path.is_file()
            restore(all_path, "all")

            measurements = []
            for codec, level in [("zstd", 3), ("xz", 9)]:
                suffix = "tar.zst" if codec == "zstd" else "tar.xz"
                for inner in ["default", "none"]:
                    path = workspace / f"{codec}-{inner}.{suffix}"
                    elapsed = []
                    peak_rss = []
                    for _ in range(args.benchmark_runs):
                        command = [binary, "backup", source, "-o", path, "--compression", codec,
                                   "--compression-level", level, "--pg-dump-compression", inner]
                        memory_file = workspace / "peak-rss.txt"
                        if Path("/usr/bin/time").is_file():
                            command = ["/usr/bin/time", "-f", "%M", "-o", memory_file, *command]
                        start = time.monotonic()
                        run(command, env=env)
                        elapsed.append(time.monotonic() - start)
                        if memory_file.is_file():
                            peak_rss.append(int(memory_file.read_text().strip()))
                    raw = extract(path, codec, workspace / f"extracted-{codec}-{inner}")
                    inner_codec = dump_codec(raw)
                    assert inner_codec == ("none" if inner == "none" else "gzip"), inner_codec
                    target = restore(path, codec + "_" + inner)
                    sql(target, "DELETE FROM fixture WHERE id = 1;")
                    cli("restore", target, "-i", path, "--file", source, "--overwrite")
                    assert fingerprint(target) == original
                    # Compression errors must be detected before --overwrite drops a DB.
                    truncated = workspace / f"truncated.{suffix}"
                    truncated.write_bytes(path.read_bytes()[:-1])
                    cli("restore", target, "-i", truncated, "--file", source, "--overwrite", expected=1)
                    assert fingerprint(target) == original
                    measurements.append({
                        "codec": codec, "level": level, "threads": 1,
                        "pg_dump_compression": inner,
                        "backup_seconds_median": round(statistics.median(elapsed), 3),
                        "archive_bytes": path.stat().st_size,
                        "temporary_dump_bytes": raw.stat().st_size,
                        "backup_peak_rss_kib": max(peak_rss) if peak_rss else None,
                    })

            for codec in ["zstd", "xz"]:
                suffix = "tar.zst" if codec == "zstd" else "tar.xz"
                path = workspace / f"parallel.{suffix}"
                cli("backup", source, "-o", path, "--compression", codec,
                    "--compression-level", "1", "--compression-threads", "2")
                run([codec, "--test", path])
                restore(path, codec + "_parallel")
                path = workspace / f"sql.{suffix}"
                cli("backup", source, "-o", path, "--compression", codec,
                    "--compression-level", "1", "--format", "sql")
                restore(path, codec + "_sql", format="sql")

            for inner in ["default", "none"]:
                path = workspace / f"raw-{inner}.bsql"
                options = [] if inner == "default" else ["--pg-dump-compression", "none"]
                cli("backup", source, "--no-archive", "-o", path, *options)
                assert dump_codec(path) == ("gzip" if inner == "default" else "none")
                # Raw files use --file only for extraction, so their actual names do not matter.
                restore(path, "raw_" + inner)

            raw_sql = workspace / "raw.sql"
            cli("backup", source, "--no-archive", "--format", "sql", "-o", raw_sql)
            restore(raw_sql, "raw_sql", format="sql")

            legacy_dir = workspace / "legacy"
            legacy_dir.mkdir()
            legacy_dump = legacy_dir / (source + ".bsql")
            cli("backup", source, "--no-archive", "-o", legacy_dump)
            legacy_archive = workspace / "legacy.tar.xz"
            run(["tar", "-cJf", legacy_archive, "-C", legacy_dir, legacy_dump.name])
            restore(legacy_archive, "legacy_xz")
            renamed = workspace / "renamed.backup"
            renamed.write_bytes((workspace / "zstd-none.tar.zst").read_bytes())
            restore(renamed, "renamed")

            failed_output = workspace / "failed.tar.zst"
            failed_output.write_bytes(b"existing backup")
            cli("backup", prefix + "_missing", "-o", failed_output, expected=1)
            assert failed_output.read_bytes() == b"existing backup"
            cli("backup", source, "-o", workspace / "wrong.tar.xz", expected=1)
            assert not (workspace / "wrong.tar.xz").exists()
            cli("backup", source, "--compression-level", "23", "-o", workspace / "invalid", expected=1)
            print("PASS failure propagation, conflicting suffixes and invalid levels", flush=True)

            report = {
                "rows": args.rows,
                "benchmark_runs": args.benchmark_runs,
                "pg_dump_version": run(["pg_dump", "--version"]).strip(),
                "measurements": measurements,
                "note": "Synthetic data; local end-to-end dump/archive times exclude upload. Temporary dump bytes exclude the final archive. RSS is GNU time's maximum process RSS, not aggregate memory across simultaneous processes.",
            }
            print(json.dumps(report, indent=2), flush=True)
            if args.report:
                args.report.write_text(json.dumps(report, indent=2) + "\n")
        finally:
            for database in reversed(databases):
                sql("postgres", f'DROP DATABASE IF EXISTS "{database}" WITH (FORCE);')


if __name__ == "__main__":
    main()
