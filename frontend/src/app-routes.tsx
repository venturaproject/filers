import type { RouteObject } from 'react-router-dom'

import SignIn from '@/pages/auth/sign-in/sign-in-2'
import ForgotPasswordPage from '@/pages/auth/forgot-password'

import DashboardRoute from '@/routes/dashboard-route'
import UsersRoute from '@/routes/users-route'
import RolesRoute from '@/routes/roles-route'
import PermissionsRoute from '@/routes/permissions-route'
import SettingsRoute from '@/routes/settings-route'
import ProcessRoute from '@/routes/process-route'

// ── Guest routes (unauthenticated only) ───────────────────────────────────────

export const guestRoutes: RouteObject[] = [
  { path: '/login', element: <SignIn canResetPassword={true} /> },
  { path: '/forgot-password', element: <ForgotPasswordPage /> },
]

// ── Private routes (authenticated only) ──────────────────────────────────────

export const privateRoutes: RouteObject[] = [
  { path: '/admin', element: <DashboardRoute /> },

  { path: '/admin/process', element: <ProcessRoute /> },
  { path: '/admin/jobs', element: <ProcessRoute /> },
  { path: '/admin/jobs/:id', element: <ProcessRoute /> },

  { path: '/admin/users', element: <UsersRoute /> },
  { path: '/admin/users/create', element: <UsersRoute /> },
  { path: '/admin/users/:id', element: <UsersRoute /> },
  { path: '/admin/users/:id/edit', element: <UsersRoute /> },

  { path: '/admin/roles', element: <RolesRoute /> },
  { path: '/admin/roles/create', element: <RolesRoute /> },
  { path: '/admin/roles/:id/edit', element: <RolesRoute /> },

  { path: '/admin/permissions', element: <PermissionsRoute /> },
  { path: '/admin/permissions/create', element: <PermissionsRoute /> },
  { path: '/admin/permissions/:id/edit', element: <PermissionsRoute /> },

  { path: '/admin/settings', element: <SettingsRoute /> },
  { path: '/admin/settings/:section', element: <SettingsRoute /> },
]
