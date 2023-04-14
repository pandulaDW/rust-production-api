#!/bin/zsh

set -x
set -eo pipefail

# install psql
if ! [ -x "$(command -v psql)" ]; then
  echo >&2 "Error: psql is not installed."
  exit 1
fi

# install sqlx
if ! [ -x "$(command -v sqlx)" ]; then
  echo >&2 "Error: sqlx is not installed."
  echo >&2 "Use:"
  echo >&2 "  cargo install --version=0.5.7 sqlx-cli --no-default-features --features postgres"
  echo >&2 "to install it."
  exit 1
fi

# Check if a custom user has been set, otherwise default to 'postgres'
RUST_DB_USER=${POSTGRES_USER:=postgres}

# Check if a custom password has been set, otherwise default to 'password'
RUST_DB_PASSWORD="${POSTGRES_PASSWORD:=password}"

# Check if a custom database name has been set, otherwise default to 'newsletter'
RUST_DB_NAME="${POSTGRES_DB:=newsletter}"

# Check if a custom port has been set, otherwise default to '5432'
RUST_DB_PORT="${POSTGRES_PORT:=5433}"

# Launch postgres using Docker
# Allow to skip Docker if a dockerized Postgres database is already running
if [[ -z "${SKIP_DOCKER}" ]]
then 
  docker run \
    -e POSTGRES_USER=${RUST_DB_USER} \
    -e POSTGRES_PASSWORD=${RUST_DB_PASSWORD} \
    -e POSTGRES_DB=${RUST_DB_NAME} \
    -p "${RUST_DB_PORT}":5433 \
    --rm postgres:11-alpine \
    postgres -N 1000 # ^ Increased maximum number of connections for testing purposes
fi

# Keep pinging Postgres until it's ready to accept commands
export PGPASSWORD="${DB_PASSWORD}"
until psql -h "localhost" -U "${DB_USER}" -p "${DB_PORT}" -d "postgres" -c '\q'; do
  >&2 echo "Postgres is still unavailable - sleeping"
  sleep 1 
done

>&2 echo "Postgres is up and running on port ${DB_PORT}!"

export DATABASE_URL=postgres://${RUST_DB_USER}:${RUST_DB_PASSWORD}@localhost:${RUST_DB_PORT}/${RUST_DB_NAME}
sqlx database create # uses DATABASE_URL
sqlx migrate run # run migrations

>&2 echo "Postgres has been migrated, ready to go!"