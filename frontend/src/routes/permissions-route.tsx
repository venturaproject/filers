import { useQuery } from '@tanstack/react-query'
import { useSearchParams, useParams, useLocation } from 'react-router-dom'
import PermissionsPage from '@/pages/permissions/index'
import CreatePermissionPage from '@/pages/permissions/create'
import EditPermissionPage from '@/pages/permissions/edit'
import { RoutePending } from '@/components/route-pending'
import NotFoundError from '@/pages/errors/not-found-error'
import { normalizePaginatedPayload } from '@/lib/api-utils'
import { permissionsApi } from '@/services/permissions-api'

export default function PermissionsRoute() {
  const [searchParams] = useSearchParams()
  const { id } = useParams()
  const location = useLocation()
  const filters = Object.fromEntries(searchParams.entries())

  const isCreate = location.pathname.endsWith('/create')
  const isEdit = !!id && location.pathname.includes(`/${id}/edit`)

  const { data: listData, isLoading: listLoading } = useQuery({
    queryKey: ['permissions', filters],
    queryFn: () => permissionsApi.list(filters),
    enabled: !isCreate && !isEdit
  })

  const { data: permissionData, isLoading: permissionLoading, isError: permissionError } = useQuery({
    queryKey: ['permissions', id],
    queryFn: () => permissionsApi.detail(id!),
    enabled: isEdit
  })

  const permissions = normalizePaginatedPayload(listData)
  const groups: string[] =
    (listData?.groups as string[] | undefined) ??
    Array.from(new Set((permissions.data || []).map((p: any) => p.name.split('.')[0] || 'other')))

  if (isEdit) {
    if (permissionLoading) return <RoutePending />
    if (permissionError || !permissionData) return <NotFoundError />
    return <EditPermissionPage permission={permissionData as any} />
  }

  if (isCreate) {
    return <CreatePermissionPage />
  }

  if (listLoading) return <RoutePending />

  return (
    <PermissionsPage
      permissions={permissions as any}
      groups={groups as string[]}
      filters={filters}
    />
  )
}
