# ── Configuration ──────────────────────────────────────────────────────────────
DB_FILE   ?= database.db
DB_URL    ?= sqlite://$(DB_FILE)
BACKEND_BIND ?= 127.0.0.1:8080

.PHONY: dev dev-backend dev-frontend build install-tools

# ── Development (both services, with reloading) ────────────────────────────────
# Requires cargo-watch: cargo install cargo-watch
# Vite (frontend) has HMR built in; the backend reloads via cargo-watch.
# Both processes are killed cleanly when you hit Ctrl-C.
dev:
	@echo "Starting backend on http://$(BACKEND_BIND) and frontend on http://localhost:5173"
	@trap 'kill 0' SIGINT SIGTERM EXIT; \
	  DATABASE_URL="$(DB_URL)" cargo watch -q -x 'run -- api-serve --bind $(BACKEND_BIND)' & \
	  (cd frontend && pnpm dev) & \
	  wait

# ── Individual services ────────────────────────────────────────────────────────
dev-backend:
	DATABASE_URL="$(DB_URL)" cargo watch -q -x 'run -- api-serve --bind $(BACKEND_BIND)'

dev-frontend:
	cd frontend && pnpm dev

# ── Production build ───────────────────────────────────────────────────────────
build:
	cd frontend && pnpm build
	cargo build --release

# ── Helpers ────────────────────────────────────────────────────────────────────
# Install cargo-watch if it isn't already present.
install-tools:
	@command -v cargo-watch >/dev/null 2>&1 || cargo install cargo-watch
	@cd frontend && pnpm install
