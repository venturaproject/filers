import { AuthenticatedLayout } from '@/layouts'
import { Main } from '@/components/layout'
import { MetricStatCard } from '@/components/metric-stat-card'
import { X, Download, Users, UserCheck, UserX, ShieldAlert } from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Input } from '@/components/ui/input'
import { DropdownMenuItem, DropdownMenuSeparator } from '@/components/ui/dropdown-menu'
import { useI18n } from '@/i18n/context'
import { useNavigate, useSearchParams } from 'react-router-dom'
import { PageProps } from '@/types'
import { getCoreRowModel, useReactTable } from '@tanstack/react-table'
import {
  DataTable,
  DataTablePagination,
  DataTableViewOptions,
  BulkSelectionBar,
} from '@/components/data-table'
import { useTableFilters } from '@/hooks/use-table-filters'
import { useBulkSelection } from '@/hooks/use-bulk-selection'
import { useColumnReorder } from '@/hooks/use-column-reorder'
import { submitBulkActionForm } from '@/lib/submit-bulk-action-form'
import { buildUsersColumns, userColumnLabels, User } from './columns'
import { useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { pathFor } from '@/lib/app-routes'
import { ConfirmDialog } from '@/components/confirm-dialog'
import { useState } from 'react'
import { usersApi } from '@/services/users-api'
import { ApiClientsTab } from './components/api-clients-tab'
import { AccessControlTabs } from './access-control-tabs'

interface UserStats {
  total: number
  activos: number
  inactivos: number
  suspendidos: number
}

interface UserFilters {
  search?: string
  status?: string
  role?: string
  [key: string]: string | undefined
}

interface Role {
  id: number
  name: string
}

interface UsersPageProps extends PageProps {
  users?: {
    data: User[]
    current_page?: number
    last_page?: number
    per_page?: number
    total?: number
  }
  filters?: UserFilters
  roles?: Role[]
  stats?: UserStats
}

const statSeries = {
  total: [6, 8, 7, 9, 8, 11, 9, 12],
  active: [5, 7, 6, 8, 7, 10, 8, 11],
  inactive: [8, 7, 8, 6, 7, 5, 6, 4],
  suspended: [6, 5, 6, 4, 5, 3, 4, 3],
}

export default function UsersPage({
  users = { data: [], current_page: 1, last_page: 1, per_page: 20, total: 0 },
  filters: initialFilters = {},
  stats = { total: 0, activos: 0, inactivos: 0, suspendidos: 0 },
}: UsersPageProps) {
  const { t } = useI18n()
  const navigateReact = useNavigate()
  const queryClient = useQueryClient()
  const [confirmDelete, setConfirmDelete] = useState<{ open: boolean; id: string | null }>({ open: false, id: null })
  const [topTab, setTopTab] = useState<'usuarios' | 'api-clients'>('usuarios')

  const pagination = {
    current_page: users.current_page ?? 1,
    last_page: users.last_page ?? 1,
    per_page: users.per_page ?? 20,
    total: users.total ?? users.data.length,
  }

  const pageIds = users.data.map((u) => u.id)

  const bulk = useBulkSelection(pageIds, pagination.total)

  const { filters, searchTerm, navigate, handleSearch, handlePageChange, handlePerPageChange, perPage } =
    useTableFilters<UserFilters>({
      basePath: pathFor('admin.users.index'),
      initialFilters: initialFilters ?? {},
      initialPerPage: pagination.per_page,
      onNavigate: bulk.clearSelection,
    })

  const handleTabChange = (value: string) => {
    const newFilters = { ...filters }
    if (value === 'active') newFilters.status = 'active'
    else if (value === 'inactive') newFilters.status = 'inactive'
    else if (value === 'suspended') newFilters.status = 'suspended'
    else delete newFilters.status
    newFilters.page = '1'
    navigate(newFilters)
  }

  const handleDeleteUser = async (id: string) => {
    setConfirmDelete({ open: true, id })
  }

  const confirmDeleteUser = async () => {
    if (!confirmDelete.id) return
    const id = confirmDelete.id
    setConfirmDelete({ open: false, id: null })
    
    try {
      await usersApi.delete(id)
      toast.success(t('user_deleted_successfully') || 'Usuario eliminado correctamente')
      queryClient.invalidateQueries({ queryKey: ['users'] })
    } catch (error: any) {
      toast.error(t('something_went_wrong') || 'Error al eliminar el usuario')
    }
  }

  const columns = buildUsersColumns({
    t,
    selectedIds: bulk.selectedIds,
    allPageSelected: bulk.allPageSelected,
    toggleSelectAll: bulk.toggleSelectAll,
    toggleSelectRow: bulk.toggleSelectRow,
    onView: (id) => navigateReact(pathFor('admin.users.show', id)),
    onEdit: (id) => navigateReact(pathFor('admin.users.edit', id)),
    onDelete: handleDeleteUser,
  })

  const { columnOrder, columnVisibility, setColumnVisibility, handleDragStart, handleDrop } =
    useColumnReorder(columns.map((c) => c.id as string))

  const table = useReactTable({
    data: users.data,
    columns,
    state: { columnOrder, columnVisibility },
    onColumnOrderChange: () => {},
    onColumnVisibilityChange: setColumnVisibility,
    getCoreRowModel: getCoreRowModel(),
  })

  const currentTab = filters.status ?? 'all'

  const countLabel = bulk.selectAllRecords
    ? `${pagination.total} ${t('users_selected_all') || 'usuarios seleccionados'}`
    : `${bulk.selectedCount} ${bulk.selectedCount === 1 ? (t('user_selected_one') || 'usuario seleccionado') : (t('users_selected_other') || 'usuarios seleccionados')}`

  return (
    <AuthenticatedLayout title={t('users') || 'Usuarios'}>
      <Main>
        <div className="grid flex-1 items-start gap-4 md:gap-8">
          <div>
            <h2 className="text-2xl font-bold tracking-tight">{t('user_list') || 'Listado de Usuarios'}</h2>
            <p className="text-muted-foreground">{t('manage_users_and_roles') || 'Gestiona los usuarios y sus permisos'}</p>
          </div>

          <AccessControlTabs />

          {/* Top-level tab switcher: above the cards */}
          <Tabs
            value={topTab}
            onValueChange={(v) => setTopTab(v as 'usuarios' | 'api-clients')}
            className="space-y-4"
          >
            <TabsList>
              <TabsTrigger value="usuarios">Usuarios internos</TabsTrigger>
              <TabsTrigger value="api-clients">Clientes API externa</TabsTrigger>
            </TabsList>

            <TabsContent value="api-clients">
              <ApiClientsTab />
            </TabsContent>

            <TabsContent value="usuarios" className="space-y-4">
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
            <MetricStatCard title={t('stat_total_users') || 'Total Usuarios'} value={stats.total} subtitle={t('stat_registered_in_system') || 'Registrados'} icon={Users} tooltip={t('stat_total_users')} trend={14.2} sparklineColor="#6366f1" sparklineData={statSeries.total} detailsLabel={t('stat_registered_in_system')} />
            <MetricStatCard title={t('status_active') || 'Activos'} value={stats.activos} subtitle={t('stat_users_active_subtitle') || 'En línea'} icon={UserCheck} tooltip={t('stat_users_active_subtitle')} trend={11.5} sparklineColor="#10b981" sparklineData={statSeries.active} detailsLabel={t('status_active')} />
            <MetricStatCard title={t('status_inactive') || 'Inactivos'} value={stats.inactivos} subtitle={t('stat_users_inactive_subtitle') || 'Fuera de línea'} icon={UserX} tooltip={t('stat_users_inactive_subtitle')} trend={-5.8} sparklineColor="#94a3b8" sparklineData={statSeries.inactive} detailsLabel={t('status_inactive')} />
            <MetricStatCard title={t('status_suspended') || 'Suspendidos'} value={stats.suspendidos} subtitle={t('stat_users_suspended_subtitle') || 'Cuentas bloqueadas'} icon={ShieldAlert} tooltip={t('stat_users_suspended_subtitle')} trend={-3.4} sparklineColor="#f87171" sparklineData={statSeries.suspended} detailsLabel={t('status_suspended')} />
          </div>
          <Tabs value={currentTab} onValueChange={handleTabChange}>
            <div className="flex items-center">
              <TabsList>
                <TabsTrigger value="all">{t('tab_todos') || 'Todos'}</TabsTrigger>
                <TabsTrigger value="active">{t('status_active') || 'Activos'}</TabsTrigger>
                <TabsTrigger value="inactive">{t('status_inactive') || 'Inactivos'}</TabsTrigger>
                <TabsTrigger value="suspended">{t('status_suspended') || 'Suspendidos'}</TabsTrigger>
              </TabsList>
              <div className="ml-auto flex items-center gap-2">
                <div className="relative">
                  <Input
                    placeholder={t('filter_placeholder') || 'Buscar...'}
                    value={searchTerm}
                    onChange={(e) => handleSearch(e.target.value)}
                    className="w-72 pr-9"
                  />
                  {searchTerm && (
                    <Button type="button" variant="ghost" size="icon" className="absolute right-1 top-1/2 h-7 w-7 -translate-y-1/2 text-muted-foreground" onClick={() => handleSearch('')}>
                      <X className="h-4 w-4" />
                    </Button>
                  )}
                </div>
                <DataTableViewOptions table={table} columnLabels={userColumnLabels(t)} />
                <Button variant="default" size="sm" className="h-9 gap-1" onClick={() => navigateReact(pathFor('admin.users.create'))}>
                  {t('add_user') || 'Añadir Usuario'}
                </Button>
              </div>
            </div>

            <BulkSelectionBar
              visible={bulk.selectedIds.size > 0}
              selectedCount={bulk.selectedCount}
              total={pagination.total}
              allPageSelected={bulk.allPageSelected}
              selectAllRecords={bulk.selectAllRecords}
              onSelectAllRecords={() => bulk.setSelectAllRecords(true)}
              onSelectPageOnly={() => bulk.setSelectAllRecords(false)}
              onClearSelection={bulk.clearSelection}
              countLabel={countLabel}
              selectAllLabel={t('select_all_users', { total: pagination.total }) || `Seleccionar los ${pagination.total} usuarios`}
              actions={
                <DropdownMenuItem
                  onClick={() =>
                    submitBulkActionForm(pathFor('admin.users.bulk-export'), bulk.selectedIds, bulk.selectAllRecords, filters)
                  }
                >
                  <Download className="mr-2 h-4 w-4" />
                  {t('export_excel') || 'Exportar Excel'}
                </DropdownMenuItem>
              }
            />

            <TabsContent value={currentTab}>
              <Card>
                <CardHeader>
                  <CardTitle>{t('all_users_title') || 'Todos los Usuarios'}</CardTitle>
                  <CardDescription>{t('all_users_description') || 'Gestiona las cuentas de usuario de la plataforma'}</CardDescription>
                </CardHeader>
                <CardContent>
                  <DataTable
                    table={table}
                    colCount={columns.length}
                    emptyMessage={t('no_results') || 'No se encontraron resultados'}
                    onDragStart={handleDragStart}
                    onDrop={handleDrop}
                  />
                </CardContent>
                <DataTablePagination
                  currentPage={pagination.current_page}
                  lastPage={pagination.last_page}
                  perPage={perPage}
                  total={pagination.total}
                  selectedCount={bulk.selectedCount}
                  onPageChange={handlePageChange}
                  onPerPageChange={handlePerPageChange}
                />
              </Card>
            </TabsContent>
          </Tabs>
            </TabsContent>
          </Tabs>
        </div>
      </Main>

      <ConfirmDialog
        open={confirmDelete.open}
        onOpenChange={(open) => setConfirmDelete(prev => ({ ...prev, open }))}
        title={t('delete_user') || 'Eliminar Usuario'}
        desc={t('confirm_delete_user') || '¿Estás seguro de que deseas eliminar este usuario? Esta acción no se puede deshacer.'}
        confirmText={t('delete') || 'Eliminar'}
        destructive
        handleConfirm={confirmDeleteUser}
      />
    </AuthenticatedLayout>
  )
}
