import type { ComponentProps } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useSearchParams, useParams, useLocation } from 'react-router-dom'
import RolesPage from '@/pages/roles/index'
import CreateRolePage from '@/pages/roles/create'
import EditRolePage from '@/pages/roles/edit'
import { RoutePending } from '@/components/route-pending'
import NotFoundError from '@/pages/errors/not-found-error'
import { normalizePaginatedPayload } from '@/lib/api-utils'
import { rolesApi } from '@/services/roles-api'
import { permissionsApi } from '@/services/permissions-api'

export default function RolesRoute() {
  const [searchParams] = useSearchParams()
  const { id } = useParams()
  const location = useLocation()
  const filters = Object.fromEntries(searchParams.entries())

  const isCreate = location.pathname.endsWith('/create')
  const isEdit = !!id && location.pathname.includes(`/${id}/edit`)

  const { data: listData, isLoading: listLoading } = useQuery({
    queryKey: ['roles', filters],
    queryFn: () => rolesApi.list(filters),
    enabled: !isCreate && !isEdit
  })

  const { data: permissionsData } = useQuery({
    queryKey: ['permissions', 'all'],
    queryFn: () => permissionsApi.all({ per_page: 1000 }),
    enabled: isCreate || isEdit
  })

  const { data: roleData, isLoading: roleLoading, isError: roleError } = useQuery({
    queryKey: ['roles', id],
    queryFn: () => rolesApi.detail(id!),
    enabled: isEdit
  })

  type PermissionLike = { id: number; name: string }
  type PermissionGroup = {
    key: string
    label: string
    permissions: Array<{ id: number; name: string; action: string; actionLabel: string }>
  }

  const groupPermissions = (permissions: PermissionLike[]): Record<string, PermissionGroup> => {
    const grouped: Record<string, PermissionGroup> = {}
    permissions.forEach(p => {
      const parts = p.name.split('.')
      const model = parts[0] || 'other'
      const action = parts[1] || p.name
      
      if (!grouped[model]) {
        grouped[model] = {
          key: model,
          label: model.charAt(0).toUpperCase() + model.slice(1),
          permissions: []
        }
      }
      
      grouped[model].permissions.push({
        id: p.id,
        name: p.name,
        action: action,
        actionLabel: action.charAt(0).toUpperCase() + action.slice(1)
      })
    })
    return grouped
  }

  const roles = normalizePaginatedPayload(listData)
  const permissions = (permissionsData ?? []) as PermissionLike[]
  const groupedPermissions = groupPermissions(permissions)

  if (isEdit) {
    if (roleLoading) return <RoutePending />
    if (roleError || !roleData) return <NotFoundError />
    const roleRecord = roleData as { permissions?: Array<{ id: number }> }
    return (
      <EditRolePage
        role={roleData as unknown as ComponentProps<typeof EditRolePage>['role']}
        permissions={permissions as unknown as ComponentProps<typeof EditRolePage>['permissions']}
        groupedPermissions={groupedPermissions as unknown as ComponentProps<typeof EditRolePage>['groupedPermissions']}
        rolePermissions={Array.isArray(roleRecord?.permissions) ? roleRecord.permissions.map((p) => p.id) : []}
      />
    )
  }

  if (isCreate) {
    return (
      <CreateRolePage
        permissions={permissions as unknown as ComponentProps<typeof CreateRolePage>['permissions']}
        groupedPermissions={groupedPermissions as unknown as ComponentProps<typeof CreateRolePage>['groupedPermissions']}
      />
    )
  }

  if (listLoading) return <RoutePending />

  return (
    <RolesPage
      roles={roles as unknown as ComponentProps<typeof RolesPage>['roles']}
      filters={filters}
      permissions={listData?.permissions ?? []}
    />
  )
}
