#!/usr/bin/env bash
# Manage a throw-away Postgres container for local API testing.
# Requires only podman — no docker-compose or podman-compose needed.
#
# Usage:
#   ./scripts/pg-dev.sh up      # start postgres + show DATABASE_URL
#   ./scripts/pg-dev.sh down    # stop and remove the container
#   ./scripts/pg-dev.sh logs    # tail container logs
#   ./scripts/pg-dev.sh build   # build the nn2 image
#   ./scripts/pg-dev.sh run     # build + start nn2 api pointing at pg
#   ./scripts/pg-dev.sh stop    # stop the nn2 api container

set -euo pipefail

PG_NAME="nn2-postgres"
API_NAME="nn2-api"
PG_USER="nn2"
PG_PASS="nn2secret"
PG_DB="nn2"
PG_PORT="5432"
API_PORT="8080"
IMAGE="nn2:dev"
DATABASE_URL="postgres://${PG_USER}:${PG_PASS}@localhost:${PG_PORT}/${PG_DB}"

cmd="${1:-help}"

pg_up() {
    if podman container exists "$PG_NAME" 2>/dev/null; then
        echo "Container '${PG_NAME}' already exists — starting it if stopped."
        podman start "$PG_NAME" 2>/dev/null || true
    else
        podman run -d \
            --name "$PG_NAME" \
            -e POSTGRES_USER="$PG_USER" \
            -e POSTGRES_PASSWORD="$PG_PASS" \
            -e POSTGRES_DB="$PG_DB" \
            -p "${PG_PORT}:5432" \
            docker.io/postgres:16-alpine
    fi

    echo -n "Waiting for Postgres to be ready"
    for _ in $(seq 1 30); do
        if podman exec "$PG_NAME" pg_isready -U "$PG_USER" -d "$PG_DB" -q 2>/dev/null; then
            echo " ready."
            break
        fi
        echo -n "."
        sleep 1
    done

    echo
    echo "DATABASE_URL=${DATABASE_URL}"
    echo
    echo "To start the API against this database:"
    echo "  DATABASE_URL='${DATABASE_URL}' cargo run -- api-serve"
    echo
    echo "Or source it:"
    echo "  export DATABASE_URL='${DATABASE_URL}'"
}

case "$cmd" in
    up)
        pg_up
        ;;
    down)
        podman rm -f "$PG_NAME" 2>/dev/null && echo "Removed ${PG_NAME}" || echo "(already gone)"
        ;;
    logs)
        podman logs -f "$PG_NAME"
        ;;
    build)
        podman build -t "$IMAGE" .
        ;;
    run)
        pg_up
        podman rm -f "$API_NAME" 2>/dev/null || true
        podman build -t "$IMAGE" .
        podman run -d \
            --name "$API_NAME" \
            -e DATABASE_URL="$DATABASE_URL" \
            --network host \
            -p "${API_PORT}:8080" \
            "$IMAGE"
        echo "API running at http://localhost:${API_PORT}/api/v1/"
        echo "Logs: podman logs -f ${API_NAME}"
        ;;
    stop)
        podman rm -f "$API_NAME" 2>/dev/null && echo "Removed ${API_NAME}" || echo "(already gone)"
        ;;
    *)
        echo "Usage: $0 {up|down|logs|build|run|stop}"
        exit 1
        ;;
esac
