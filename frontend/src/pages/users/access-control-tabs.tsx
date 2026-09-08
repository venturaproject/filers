import { useLocation, useNavigate } from 'react-router-dom'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { pathFor } from '@/lib/app-routes'
import { usePermission } from '@/hooks/use-permission'
import { useI18n } from '@/i18n/context'

/**
 * Shared sub-navigation for the Access Control area. The sidebar only links to
 * Users — Roles and Permissions are reached from here instead, as tabs within
 * the same section, rather than as separate top-level nav entries.
 */
export function AccessControlTabs() {
  const { t } = useI18n()
  const { can } = usePermission()
  const navigate = useNavigate()
  const { pathname } = useLocation()

  const current = pathname.startsWith('/admin/roles')
    ? 'roles'
    : pathname.startsWith('/admin/permissions')
      ? 'permissions'
      : 'users'

  const tabs = [
    { value: 'users', label: t('users') || 'Usuarios', permission: 'users.view' },
    { value: 'roles', label: t('roles') || 'Roles', permission: 'roles.view' },
    { value: 'permissions', label: t('permissions') || 'Permisos', permission: 'permissions.view' },
  ].filter((tab) => can(tab.permission))

  if (tabs.length < 2) return null

  const handleChange = (value: string) => {
    if (value === current) return
    navigate(pathFor(`admin.${value}.index`))
  }

  return (
    <Tabs value={current} onValueChange={handleChange}>
      <TabsList>
        {tabs.map((tab) => (
          <TabsTrigger key={tab.value} value={tab.value}>
            {tab.label}
          </TabsTrigger>
        ))}
      </TabsList>
    </Tabs>
  )
}
