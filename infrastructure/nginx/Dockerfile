# ─────────────────────────────────────────────────────────────────────────────
# Edge image: builds the frontend and serves it via nginx, proxying /api.
# Build context is the repo root (see compose.prod.yml).
# ─────────────────────────────────────────────────────────────────────────────

# ---- build frontend ----
FROM node:22-alpine AS build

RUN corepack enable
WORKDIR /app

COPY frontend/package.json frontend/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile

COPY frontend/ ./
# Skip the `tsc` step from `pnpm build`; the bundle is what we ship.
RUN pnpm exec vite build

# ---- nginx ----
FROM nginx:1.27-alpine

COPY infrastructure/nginx/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=build /app/dist /srv/frontend

# Bundle a self-signed cert so the image boots on HTTPS out of the box.
# Mount a real cert over /etc/nginx/certs in production (see compose.prod.yml).
RUN apk add --no-cache openssl \
    && mkdir -p /etc/nginx/certs \
    && openssl req -x509 -newkey rsa:2048 -nodes -days 365 \
        -keyout /etc/nginx/certs/privkey.pem \
        -out /etc/nginx/certs/fullchain.pem \
        -subj "/CN=localhost" \
    && apk del openssl

EXPOSE 80 443
