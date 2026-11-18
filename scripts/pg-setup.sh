#!/usr/bin/env bash
set -euo pipefail

DB_USER="${QUESTFORGE_DB_USER:-questforge}"
DB_PASS="${QUESTFORGE_DB_PASS:-questforge}"
DB_NAME="${QUESTFORGE_DB_NAME:-questforge}"

sudo -u postgres psql -v ON_ERROR_STOP=1 <<SQL
DO \$\$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = '${DB_USER}') THEN
    CREATE ROLE ${DB_USER} WITH LOGIN PASSWORD '${DB_PASS}';
  END IF;
END
\$\$;
SQL

if ! sudo -u postgres psql -tAc "SELECT 1 FROM pg_database WHERE datname='${DB_NAME}'" | grep -q 1; then
  sudo -u postgres createdb -O "${DB_USER}" "${DB_NAME}"
fi

echo "PostgreSQL ready: postgres://${DB_USER}:****@127.0.0.1:5432/${DB_NAME}"
