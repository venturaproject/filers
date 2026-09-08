import { useQuery } from '@tanstack/react-query'
import { useSearchParams, useParams, useLocation } from 'react-router-dom'
import UsersPage from '@/pages/users/index'
import CreateUserPage from '@/pages/users/create'
import EditUserPage from '@/pages/users/edit'
import ShowUserPage from '@/pages/users/show'
import { RoutePending } from '@/components/route-pending'
import NotFoundError from '@/pages/errors/not-found-error'
import { normalizePaginatedPayload } from '@/lib/api-utils'
import { usersApi } from '@/services/users-api'
import { rolesApi } from '@/services/roles-api'

export default function UsersRoute() {
  const [searchParams] = useSearchParams()
  const { id } = useParams()
  const location = useLocation()
  const filters = Object.fromEntries(searchParams.entries())

  const isCreate = location.pathname.endsWith('/create')
  const isEdit = !!id && location.pathname.includes(`/${id}/edit`)
  const isShow = !!id && !isEdit && location.pathname.includes(`/users/${id}`)

  const { data: listData, isLoading: listLoading } = useQuery({
    queryKey: ['users', filters],
    queryFn: () => usersApi.list(filters),
    enabled: !isCreate && !isEdit && !isShow
  })

  const { data: rolesData } = useQuery({
    queryKey: ['roles', 'all'],
    queryFn: () => rolesApi.all({ per_page: 1000 }),
    enabled: isCreate || isEdit
  })

  const { data: userData, isLoading: userLoading, isError: userError } = useQuery({
    queryKey: ['users', id],
    queryFn: () => usersApi.detail(id!),
    enabled: !!id
  })

  const users = normalizePaginatedPayload(listData)
  const roles = rolesData ?? []

  if (isEdit) {
    if (userLoading) return <RoutePending />
    if (userError || !userData) return <NotFoundError />
    return <EditUserPage user={userData as any} roles={roles as any} />
  }

  if (isCreate) {
    return <CreateUserPage roles={roles as any} />
  }

  if (isShow) {
    if (userLoading) return <RoutePending />
    if (userError || !userData) return <NotFoundError />
    return <ShowUserPage user={userData as any} />
  }

  if (listLoading) return <RoutePending />

  return (
    <UsersPage
      users={users as any}
      stats={listData?.stats ?? { total: 0, activos: 0, inactivos: 0, suspendidos: 0 }}
      filters={filters}
      roles={roles as any}
    />
  )
}
