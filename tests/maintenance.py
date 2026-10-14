#!/usr/bin/env python3
"""Run maintenance regressions against an isolated Docker PostgreSQL fixture.

The fixture is created/removed by this script. No existing DB, container,
passfile or user credentials are used. Requires Docker, PostgreSQL 18 clients
(pg_dump, pg_restore, psql) matching the fixture, and OpenSSL.
"""

import argparse
import contextlib
import http.server
import json
import os
from pathlib import Path
import subprocess
import tempfile
import threading
import time
import uuid
from urllib.parse import urlsplit, parse_qs

ROOT = Path(__file__).resolve().parents[1]


def run(args, *, env=None, cwd=ROOT, input=None, ok=True):
    result = subprocess.run([str(a) for a in args], cwd=cwd, env=env, input=input,
                            text=True, capture_output=True, timeout=180)
    if (result.returncode == 0) != ok:
        # Never include environment or secret-bearing command arguments.
        raise AssertionError(f"{Path(str(args[0])).name}: unexpected exit {result.returncode}: {result.stderr[-1500:]}")
    return result


def quote(name):
    return '"' + name.replace('"', '""') + '"'


@contextlib.contextmanager
def postgres_fixture(workspace):
    name = "pg-actions-tests-" + uuid.uuid4().hex[:12]
    run(["docker", "run", "--detach", "--rm", "--name", name, "--publish", "127.0.0.1::5432",
         "--env", "POSTGRES_PASSWORD=fixture-only", "postgres:18.6-bookworm"])
    try:
        port = run(["docker", "port", name, "5432"]).stdout.strip().rsplit(":", 1)[1]
        env = {k: v for k, v in os.environ.items() if not k.startswith(("PG", "S3"))
               and k not in ("LOG_FILE", "USE_JSON_LOGGING", "OVERWRITE", "FILE_NAME", "RESTORE_LOCATION", "BACKUP_LOCATION", "FORMAT", "NO_ARCHIVE", "JOBS", "KEEP_TEMP", "KEEP_DUMP", "COMPRESSION", "COMPRESSION_LEVEL", "COMPRESSION_THREADS", "PG_DUMP_COMPRESSION")}
        home = workspace / "home"
        home.mkdir()
        env.update(HOME=str(home), PGHOST="127.0.0.1", PGPORT=port, PGUSER="postgres", PGPASSWORD="fixture-only",
                   PG_HOSTNAME="127.0.0.1", PG_PORT=port, PG_SUPERUSER="postgres", PG_PASS="fixture-only")
        for _ in range(60):
            result = subprocess.run(["psql", "-X", "-At", "-d", "postgres", "-c", "SELECT 1"], env=env, text=True, capture_output=True)
            if result.returncode == 0:
                break
            time.sleep(0.25)
        else:
            raise AssertionError("Disposable PostgreSQL did not become ready")
        yield name, env
    finally:
        run(["docker", "stop", "--time", "5", name])


class ObjectStore(http.server.BaseHTTPRequestHandler):
    state = None

    def log_message(self, *_):
        pass

    def reply(self, status, body=b"", etag=False):
        self.send_response(status)
        self.send_header("Content-Length", str(len(body)))
        if etag:
            self.send_header("ETag", '"fixture-etag"')
        self.end_headers()
        self.wfile.write(body)

    def consume(self):
        length = int(self.headers.get("Content-Length", "0"))
        data = self.rfile.read(length)
        assert len(data) == length
        return data

    def do_PUT(self):
        state = self.state
        data = self.consume()
        params = parse_qs(urlsplit(self.path).query)
        if "partNumber" in params:
            state["part_attempts"] += 1
            if state["mode"] == "part-failure" or (state["mode"] == "retry" and state["part_attempts"] == 1):
                self.reply(500, b"<Error><Code>InternalError</Code></Error>")
                return
            state["parts"][int(params["partNumber"][0])] = data
        else:
            state["puts"] += 1
            state["object"] = data
        self.reply(200, etag=True)

    def do_POST(self):
        state = self.state
        self.consume()
        params = parse_qs(urlsplit(self.path).query, keep_blank_values=True)
        if "uploads" in params:
            state["creates"] += 1
            self.reply(200, b'<InitiateMultipartUploadResult><Bucket>fixture</Bucket><Key>backup</Key><UploadId>fixture-upload</UploadId></InitiateMultipartUploadResult>')
        elif "uploadId" in params:
            state["completes"] += 1
            if state["mode"] == "complete-failure":
                self.reply(500, b"<Error><Code>InternalError</Code></Error>")
            else:
                state["object"] = b"".join(state["parts"][key] for key in sorted(state["parts"]))
                self.reply(200, b'<CompleteMultipartUploadResult><Bucket>fixture</Bucket><Key>backup</Key><ETag>fixture-etag</ETag></CompleteMultipartUploadResult>')
        else:
            self.reply(400)

    def do_DELETE(self):
        self.state["aborts"] += 1
        self.reply(204)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--docker", action="store_true", required=True)
    parser.add_argument("--binary", type=Path, default=ROOT / "target/debug/pg_actions")
    options = parser.parse_args()
    binary = options.binary.resolve()
    with tempfile.TemporaryDirectory(prefix="pg-actions-maintenance-") as temporary:
        workspace = Path(temporary)
        with postgres_fixture(workspace) as (container, env):
            def sql(database, text, custom=None):
                return run(["psql", "-X", "--set=ON_ERROR_STOP=1", "-At", "-d", "dbname='" + database.replace("\\", "\\\\").replace("'", "\\'") + "'"], env=custom or env, input=text).stdout.strip()

            def cli(*args, ok=True, custom=None):
                return run([binary, *args], env=custom or env, cwd=workspace, ok=ok)

            def no_stages():
                assert sql("postgres", "SELECT count(*) FROM pg_database WHERE datname LIKE 'pg_actions_stage_%' OR datname LIKE 'pg_actions_previous_%'") == "0"

            # Utilities must work with no connection/password/passfile.
            utility_env = env.copy()
            for key in ("PG_PASS", "PGPASSWORD", "PG_SUPERUSER", "PG_PORT"):
                utility_env.pop(key, None)
            utility_env["PGPASSFILE"] = str(workspace / "does-not-exist")
            assert cli("completions", "bash", custom=utility_env).stdout
            cli("check-tools", custom=utility_env)
            failed_tool = workspace / "pg_dump"
            failed_tool.write_text('#!/bin/sh\nprintf "pg_dump (PostgreSQL) 18.0\\n"\nexit 7\n')
            failed_tool.chmod(0o700)
            cli("--pg-dump", failed_tool, "check-tools", custom=utility_env, ok=False)
            assert cli("backup", "postgres", "--pg-restore", workspace / "missing", "--psql", workspace / "missing", "-o", "utility.sql", "--no-archive", "-f", "sql").returncode == 0
            print("PASS utilities and command-specific/status-aware tool checks", flush=True)

            # Ordered wildcard/escaped passfile input is read-only; explicit creds win.
            passfile = workspace / "input.pgpass"
            contents = f"# retained comment\n127.0.0.1:*:*:postgres:wrong-password\n127.0.0.1:{env['PGPORT']}:postgres:postgres:fixture-only\n"
            passfile.write_text(contents)
            passfile.chmod(0o600)
            explicit_env = env.copy()
            explicit_env["PGPASSFILE"] = str(passfile)
            cli("database", "list", "-vv", "-o", "json", custom=explicit_env)
            assert passfile.read_text() == contents
            passfile.write_text(f"# retained\n127.0.0.1:*:*:postgres:fixture-only\n")
            pass_env = explicit_env.copy()
            pass_env.pop("PG_PASS")
            pass_env.pop("PGPASSWORD")
            cli("database", "list", "-o", "json", custom=pass_env)
            assert passfile.read_text().startswith("# retained")
            print("PASS passfile wildcard/order/explicit precedence without mutation", flush=True)

            owner = 'Owner-"Mixed'
            secret = "Sentinel:punctuation'\\;password"
            log = workspace / "private.log"
            result = cli("--log-file", log, "-vv", "user", "create", owner, "--password", secret)
            assert "Sentinel" not in result.stdout + result.stderr + log.read_text()
            assert "fixture-only" not in result.stderr + log.read_text()
            assert log.stat().st_mode & 0o777 == 0o600
            cli("user", "update", owner, "--password", secret + "2", "--no-login", "true", "-vv", "--log-file", log)
            assert "Sentinel" not in log.read_text()
            generated = json.loads(cli("user", "create", "generated_owner", "-vv", "--log-file", log).stdout)
            assert generated["password"] not in log.read_text()
            generated_env = env.copy()
            generated_env.update(PGUSER="generated_owner", PGPASSWORD=generated["password"])
            assert sql("postgres", "SELECT 1", generated_env) == "1"
            json_log = workspace / "private-json.log"
            cli("user", "update", owner, "--password", secret + "2", "--no-login", "false", "-vv", "--use-json-logging", "--log-file", json_log)
            assert "Sentinel" not in json_log.read_text()
            for line in json_log.read_text().splitlines():
                assert isinstance(json.loads(line), dict)
            owner_passfile = workspace / "owner.pgpass"
            escaped_password = (secret + "2").replace("\\", "\\\\").replace(":", "\\:")
            owner_passfile.write_text(f"127.0.0.1:*:*:{owner}:{escaped_password}\n")
            owner_passfile.chmod(0o600)
            owner_env = env.copy()
            owner_env.pop("PG_PASS")
            owner_env.pop("PGPASSWORD")
            owner_env.update(PG_SUPERUSER=owner, PGPASSFILE=str(owner_passfile))
            cli("database", "list", custom=owner_env)
            print("PASS credential redaction, private logs and one-time generated credentials", flush=True)

            source = "source_db"
            sql("postgres", f"CREATE DATABASE {quote(source)}")
            schema = 'Case-"schema'
            sql(source, f"""
                CREATE SCHEMA {quote(schema)};
                CREATE TABLE {quote(schema)}.items(id serial PRIMARY KEY, value text);
                INSERT INTO {quote(schema)}.items(value) VALUES ('retained');
                CREATE VIEW {quote(schema)}.view_items AS SELECT * FROM {quote(schema)}.items;
                CREATE MATERIALIZED VIEW {quote(schema)}.material_items AS SELECT * FROM {quote(schema)}.items;
                CREATE SEQUENCE {quote(schema)}.standalone;
                CREATE TYPE {quote(schema)}.mood AS ENUM ('happy');
                CREATE DOMAIN {quote(schema)}.positive AS int CHECK (VALUE>0);
                CREATE TYPE {quote(schema)}.pair AS (x int, y text);
                CREATE FUNCTION {quote(schema)}.answer() RETURNS int LANGUAGE sql AS 'SELECT 42';
                CREATE PROCEDURE {quote(schema)}.noop() LANGUAGE sql AS 'SELECT 1';
                CREATE EXTENSION hstore;
            """)
            cli("database", "update", source, ok=False)
            cli("database", "update", source, "--new-owner", owner)
            assert sql(source, "SELECT count(*) FROM pg_class c JOIN pg_namespace n ON c.relnamespace=n.oid WHERE n.nspname=" + "E'" + schema.replace("'", "''") + "' AND c.relkind IN ('r','v','m','S') AND pg_get_userbyid(c.relowner)<>" + "E'" + owner.replace("'", "''") + "'") == "0"
            assert sql(source, "SELECT pg_get_userbyid(extowner) FROM pg_extension WHERE extname='hstore'") == "postgres"
            schema_literal = "E'" + schema.replace("'", "''") + "'"
            owner_literal = "E'" + owner.replace("'", "''") + "'"
            assert sql(source, "SELECT count(*) FROM pg_type t JOIN pg_namespace n ON t.typnamespace=n.oid WHERE n.nspname=" + schema_literal + " AND pg_get_userbyid(t.typowner)<>" + owner_literal) == "0"
            assert sql(source, "SELECT count(*) FROM pg_proc p JOIN pg_namespace n ON p.pronamespace=n.oid WHERE n.nspname=" + schema_literal + " AND pg_get_userbyid(p.proowner)<>" + owner_literal) == "0"
            cli("user", "grant", "generated_owner", "--database", source, "--schema", schema, "--privileges", "read-only")
            attack = 'missing"; DROP DATABASE source_db; --'
            cli("user", "grant", "generated_owner", "--database", source, "--schema", attack, "--privileges", "read-only", ok=False)
            assert sql("postgres", "SELECT count(*) FROM pg_database WHERE datname='source_db'") == "1"
            print("PASS quoted SQL identifiers/passwords and complete multi-schema ownership", flush=True)

            dump = workspace / "source.bsql"
            cli("backup", source, "--no-archive", "-o", dump)
            cli("restore", "destination", "-i", dump, "--owner", owner)
            assert sql("destination", f"SELECT value FROM {quote(schema)}.items") == "retained"
            bad_sql = workspace / "bad.sql"
            bad_sql.write_text("CREATE TABLE would_be_partial(id int);\nthis is invalid sql;\n")
            cli("restore", "destination", "-i", bad_sql, "-f", "sql", "--overwrite", ok=False)
            assert sql("destination", f"SELECT value FROM {quote(schema)}.items") == "retained"
            assert sql("destination", "SELECT to_regclass('would_be_partial') IS NULL") == "t"
            bad_dump = workspace / "bad.bsql"
            bad_dump.write_text("not a custom dump")
            cli("restore", "destination", "-i", bad_dump, "--overwrite", ok=False)
            cli("restore", "destination", "-i", dump, "--overwrite", "--owner", "nonexistent_owner", ok=False)
            assert sql("destination", f"SELECT value FROM {quote(schema)}.items") == "retained"
            cli("clone", "-d", source, "-n", source, "--overwrite", "--pg-hostname2", "localhost", "--pg-port2", env["PGPORT"], "--pg-superuser2", "postgres", "--pg-password2", "fixture-only", ok=False)
            cli("clone", "-d", source, "-n", "cloned", "--new-owner", owner)
            sql("destination", f"DELETE FROM {quote(schema)}.items")
            cli("restore", "destination", "-i", dump, "--overwrite", "--owner", owner)
            assert sql("destination", f"SELECT value FROM {quote(schema)}.items") == "retained"
            no_stages()
            for codec in ("zstd", "xz"):
                archive = workspace / ("roundtrip.tar.zst" if codec == "zstd" else "roundtrip.tar.xz")
                cli("backup", source, "--compression", codec, "--compression-level", "1", "--format", "sql", "-o", archive)
                cli("restore", "roundtrip_" + codec, "-i", archive, "--format", "sql", "--file", source)
                assert sql("roundtrip_" + codec, f"SELECT value FROM {quote(schema)}.items") == "retained"
            retained = cli("restore", "destination", "-i", bad_sql, "--format", "sql", "--overwrite", "--keep-temp", ok=False)
            marker = "Temporary files kept at "
            retained_path = Path(next(line.split(marker, 1)[1] for line in retained.stderr.splitlines() if marker in line))
            assert retained_path.is_dir()
            # Remove only the private path explicitly returned by this invocation.
            import shutil
            shutil.rmtree(retained_path)
            no_stages()
            print("PASS staged restore/clone, rollback on SQL errors and destructive preflight", flush=True)

            unusual = "../Outside/Case"
            sql("postgres", f"CREATE DATABASE {quote(unusual)}")
            sql(unusual, "CREATE TABLE marker(value int); INSERT INTO marker VALUES (7)")
            cli("backup", unusual, "-o", "portable")
            cli("restore", "mapped_destination", "-i", "portable.tar.zst", "--file", unusual)
            assert sql("mapped_destination", "SELECT value FROM marker") == "7"
            assert not (workspace.parent / "Outside").exists()
            conninfo_name = "dbname=postgres"
            sql("postgres", f"CREATE DATABASE {quote(conninfo_name)}")
            sql(conninfo_name, "CREATE TABLE literal_marker(value int); INSERT INTO literal_marker VALUES (9)")
            cli("backup", conninfo_name, "-o", "conninfo")
            cli("restore", "literal_destination", "-i", "conninfo.tar.zst", "--file", conninfo_name)
            assert sql("literal_destination", "SELECT value FROM literal_marker") == "9"
            assert not json.loads(cli("database", "list", "--query", "a_name_no_database_has_6729", "-o", "json").stdout)
            print("PASS safe archive filenames, relative outputs and empty JSON results", flush=True)

            # The implicit v2 policy preserves unverified TLS/plaintext fallback.
            implicit = cli("database", "list", "-o", "json")
            assert isinstance(json.loads(implicit.stdout), list)
            assert "In v3.0 the default will change to verify-full" in implicit.stderr
            for explicit in ("prefer", "disable"):
                result = cli("--pg-ssl-mode", explicit, "database", "list")
                assert "In v3.0" not in result.stderr
            explicit_env = env.copy()
            explicit_env["PG_SSL_MODE"] = "prefer"
            assert "In v3.0" not in cli("database", "list", custom=explicit_env).stderr
            # Global arguments also propagate when supplied after subcommands.
            assert "In v3.0" not in cli("database", "list", "--pg-ssl-mode", "prefer").stderr
            plaintext_rejected = env.copy()
            plaintext_rejected["PG_SSL_MODE"] = "verify-full"
            cli("database", "list", custom=plaintext_rejected, ok=False)
            # Generate a private CA/server certificate and enable fixture TLS.
            ca = workspace / "ca.pem"
            ca_key = workspace / "ca.key"
            key = workspace / "server.key"
            csr = workspace / "server.csr"
            certificate = workspace / "server.crt"
            run(["openssl", "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-keyout", ca_key, "-out", ca, "-days", "1", "-subj", "/CN=pg-actions-fixture-ca"])
            run(["openssl", "req", "-newkey", "rsa:2048", "-nodes", "-keyout", key, "-out", csr, "-subj", "/CN=localhost"])
            extensions = workspace / "extensions"
            extensions.write_text("subjectAltName=DNS:localhost\nextendedKeyUsage=serverAuth\n")
            run(["openssl", "x509", "-req", "-in", csr, "-CA", ca, "-CAkey", ca_key, "-CAcreateserial", "-out", certificate, "-days", "1", "-extfile", extensions])
            run(["docker", "cp", key, container + ":/tmp/fixture.key"])
            run(["docker", "cp", certificate, container + ":/tmp/fixture.crt"])
            run(["docker", "exec", "--user", "root", container, "sh", "-c", "chown postgres:postgres /tmp/fixture.key /tmp/fixture.crt && chmod 600 /tmp/fixture.key"])
            sql("postgres", "ALTER SYSTEM SET ssl_cert_file='/tmp/fixture.crt'; ALTER SYSTEM SET ssl_key_file='/tmp/fixture.key'; ALTER SYSTEM SET ssl='on'; SELECT pg_reload_conf();")
            time.sleep(0.5)
            hba = sql("postgres", "SHOW hba_file")
            run(["docker", "exec", "--user", "postgres", container, "sh", "-c",
                 'printf "hostnossl all all 0.0.0.0/0 reject\\nhostnossl all all ::0/0 reject\\n" > "$1.new"\ncat "$1" >> "$1.new"\nmv "$1.new" "$1"', "sh", hba])
            sql("postgres", "SELECT pg_reload_conf()")
            time.sleep(0.5)
            # Force TLS with an untrusted certificate and mismatched hostname:
            # the v2 default must still work in Rust and both child clients.
            cli("database", "list")
            cli("backup", source, "--no-archive", "-o", "unverified.bsql")
            cli("restore", "unverified_destination", "-i", "unverified.bsql")
            assert sql("unverified_destination", f"SELECT value FROM {quote(schema)}.items") == "retained"
            cli("--pg-ssl-mode", "disable", "database", "list", ok=False)
            verified = env.copy()
            ca.write_text("# Fixture CA bundle\n\n" + ca.read_text())
            verified.update(PG_HOSTNAME="localhost", PG_SSL_MODE="verify-full", PG_SSL_ROOT_CERT=str(ca))
            cli("database", "list", custom=verified)
            wrong_host = verified.copy()
            wrong_host["PG_HOSTNAME"] = "127.0.0.1"
            cli("database", "list", custom=wrong_host, ok=False)
            untrusted = verified.copy()
            untrusted.pop("PG_SSL_ROOT_CERT")
            cli("database", "list", custom=untrusted, ok=False)
            cli("backup", source, "--no-archive", "-o", "tls.bsql", custom=verified)
            cli("restore", "tls_destination", "-i", "tls.bsql", custom=verified)
            cli("backup", source, "--no-archive", "-o", "wrong-host.bsql", custom=wrong_host, ok=False)
            print("PASS implicit/explicit TLS policy, non-TLS fallback, unverified TLS and trusted TLS with libpq child parity", flush=True)

            # Exercise the actual Rusoto wire requests against a controllable stub.
            state = dict(mode="success", puts=0, creates=0, part_attempts=0, completes=0, aborts=0, parts={}, object=b"")
            ObjectStore.state = state
            server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), ObjectStore)
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()
            try:
                s3env = env.copy()
                s3env.update(S3_ENDPOINT=f"http://127.0.0.1:{server.server_port}", S3_ACCESS_KEY="fixture-access", S3_SECRET_KEY="fixture-secret", S3_BUCKET="fixture", S3_PREFIX="backup", S3_REGION="fixture")
                cli("backup", source, custom=s3env)
                assert state["puts"] == 1 and state["creates"] == 0
                sql(source, "CREATE TABLE large_payload(value text); INSERT INTO large_payload SELECT repeat('x', 1024) FROM generate_series(1, 12000)")
                for mode in ("retry", "part-failure", "complete-failure"):
                    state.update(mode=mode, puts=0, creates=0, part_attempts=0, completes=0, aborts=0, parts={}, object=b"")
                    result = cli("backup", source, "--no-archive", "-f", "sql", custom=s3env, ok=(mode == "retry"))
                    assert "fixture-secret" not in result.stdout + result.stderr
                    assert state["creates"] == 1
                    if mode == "retry":
                        assert state["aborts"] == 0 and state["part_attempts"] == 3 and len(state["object"]) > 10 * 1024 * 1024
                    else:
                        assert state["aborts"] == 1
                    if mode == "part-failure":
                        assert state["part_attempts"] == 3
                    if mode == "complete-failure":
                        assert state["completes"] == 3
                print("PASS streamed multipart retry, part/completion failure abort and small PUT", flush=True)
            finally:
                server.shutdown()
                server.server_close()

            no_stages()
            print("All maintenance regression groups passed", flush=True)


if __name__ == "__main__":
    main()
