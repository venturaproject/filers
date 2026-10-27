# Frontend

## Stack and entrypoints

The administration UI is a TypeScript React single-page application built by
Vite. Dependencies are managed with pnpm; `frontend/pnpm-lock.yaml` is the
reproducible dependency lockfile. Tailwind CSS and Radix-based UI components
provide styling and accessible primitives. TanStack Query manages server state;
TanStack Table powers data tables; React Router handles routes.

```text
frontend/src/
├── app.tsx / app-routes.tsx         app shell and route declarations
├── providers.tsx                    global providers
├── routes/                          page-level route wrappers
├── pages/                           auth, dashboard, users, processing, etc.
├── components/                      shared layout and UI components
├── services/                         API clients by feature
├── lib/                              HTTP, auth, query and utility modules
├── context/ and i18n/                global UI state and translations
└── config/                           Vite environment configuration
```

## Route and UI structure

Unauthenticated routes include sign-in and password recovery. Private routes
cover the admin dashboard, processing/OCR/jobs, users, roles, permissions, and
settings. Route declarations are centralized in `src/app-routes.tsx`; feature
wrappers under `src/routes/` mount their page UI.

`src/providers.tsx` composes the query client, router query-string adapter,
translations, theme/search contexts, tooltips, notifications, and development
query devtools. Feature components should use shared providers and components
rather than creating parallel global state or duplicate API clients.

## API and authentication integration

`src/lib/axios.ts` configures the shared Axios client and its auth interceptors.
It sends cookies, obtains write-request headers from the selected auth adapter,
and coordinates a single refresh while queuing/replaying concurrent requests.
Feature-level API wrappers live in `src/services/` and keep transport details
out of presentation components.

Authentication/authorization must still be enforced by the backend. A hidden
button or protected client route is not an access-control mechanism.

## Configuration and commands

Vite exposes only `VITE_*` variables to browser code. See
`src/types/vite-env.d.ts` and `src/config/env.ts`; never put secrets in frontend
environment variables because values are embedded in client assets. In Docker
development, the browser uses Nginx as the same-origin API entrypoint by
default.

```bash
cd frontend
pnpm install --frozen-lockfile
pnpm dev
pnpm lint
pnpm tsc
pnpm build
```

The Docker development service mounts the source and `node_modules` volume and
runs Vite with hot reload. Production builds are served as static assets by
Nginx; the Vite development server is not part of the production stack.
