import { AuthenticatedLayout } from "@/layouts"
import { MoreHorizontal, PlusCircle, Edit, Trash2, Lock, Users } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { Input } from "@/components/ui/input"
import { Main } from "@/components/layout"
import { useState } from "react"
import { ConfirmDialog } from "@/components/confirm-dialog"
import { Permission, PageProps, PaginatedData } from "@/types"
import { useNavigate } from "react-router-dom"
import { useI18n } from "@/i18n/context"
import { toast } from "sonner"
import { useQueryClient } from "@tanstack/react-query"
import {
  Pagination,
  PaginationContent,
  PaginationItem,
  PaginationLink,
  PaginationNext,
  PaginationPrevious,
} from "@/components/ui/pagination"
import { pathFor } from "@/lib/app-routes"
import { permissionsApi } from "@/services/permissions-api"
import { AccessControlTabs } from "@/pages/users/access-control-tabs"

interface PermissionsPageProps extends PageProps {
  permissions: PaginatedData<Permission>
  groups?: string[]
  filters?: { search?: string; group?: string }
}

export default function PermissionsIndex({ permissions, groups: initialGroups = [], filters: initialFilters = {} }: PermissionsPageProps) {
  const { t } = useI18n()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [searchTerm, setSearchTerm] = useState(initialFilters?.search || "")
  const [groupFilter, setGroupFilter] = useState(initialFilters?.group || "")
  const [confirmDelete, setConfirmDelete] = useState<{ open: boolean; id: number | null }>({ open: false, id: null })

  const handleSearch = (value: string) => {
    setSearchTerm(value)
    navigate(pathFor('admin.permissions.index', { search: value || undefined, group: groupFilter || undefined }), { replace: true })
  }

  const handleGroupFilter = (value: string) => {
    setGroupFilter(value)
    navigate(pathFor('admin.permissions.index', { search: searchTerm || undefined, group: value === 'all' ? undefined : value }), { replace: true })
  }

  const handleDelete = (permission: Permission) => {
    setConfirmDelete({ open: true, id: permission.id })
  }

  const confirmDeletePermission = async () => {
    if (!confirmDelete.id) return
    const permId = confirmDelete.id
    setConfirmDelete({ open: false, id: null })
    
    try {
      await permissionsApi.delete(permId)
      toast.success(t('permission_deleted') || 'Permission deleted successfully.')
      queryClient.invalidateQueries({ queryKey: ['permissions'] })
    } catch (error: any) {
      toast.error(t('error_deleting_permission') || 'Error deleting permission.')
    }
  }

  const handlePageChange = (page: number) => {
    navigate(pathFor('admin.permissions.index', { ...initialFilters, page }), { replace: true })
  }

  const getGroupFromName = (name: string) => name.split('.')[0] || 'other'

  const generatePageNumbers = () => {
    const pages: (number | string)[] = []
    const delta = 2
    const rangeStart = Math.max(2, permissions.current_page - delta)
    const rangeEnd = Math.min(permissions.last_page - 1, permissions.current_page + delta)

    if (permissions.last_page > 1) pages.push(1)
    if (rangeStart > 2) pages.push('...')
    for (let i = rangeStart; i <= rangeEnd; i++) {
      if (i !== 1 && i !== permissions.last_page) pages.push(i)
    }
    if (rangeEnd < permissions.last_page - 1) pages.push('...')
    if (permissions.last_page > 1 && permissions.last_page !== 1) pages.push(permissions.last_page)

    return pages
  }

  const groups = initialGroups.length > 0 ? initialGroups : Array.from(new Set((permissions.data || []).map((p) => getGroupFromName(p.name)))).filter(Boolean)

  return (
    <AuthenticatedLayout title={t('permissions') || 'Permissions'}>
      <Main>
        <div className="grid flex-1 items-start gap-4 md:gap-8">
          <AccessControlTabs />
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <div>
                  <CardTitle className="flex items-center gap-2">
                    <Lock className="h-5 w-5" />
                    {t('permissions') || 'Permissions'}
                  </CardTitle>
                  <CardDescription>
                    {t('manage_permissions') || 'Manage application permissions.'}
                  </CardDescription>
                </div>
                <div className="flex items-center gap-2">
                  <Input
                    placeholder={t('search_permissions') || 'Search permissions...'}
                    value={searchTerm}
                    onChange={(e) => handleSearch(e.target.value)}
                    className="w-48"
                  />
                  <Input
                    placeholder={t('filter_by_group') || 'Filter by group...'}
                    value={groupFilter}
                    onChange={(e) => handleGroupFilter(e.target.value)}
                    className="w-32"
                  />
                  <Button
                    size="sm"
                    className="gap-1"
                    onClick={() => navigate(pathFor('admin.permissions.create'))}
                  >
                    <PlusCircle className="h-4 w-4" />
                    {t('add_permission') || 'Add Permission'}
                  </Button>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>{t('name') || 'Name'}</TableHead>
                    <TableHead>{t('group') || 'Group'}</TableHead>
                    <TableHead>{t('action') || 'Action'}</TableHead>
                    <TableHead>{t('guard') || 'Guard'}</TableHead>
                    <TableHead>
                      <span className="sr-only">{t('actions') || 'Actions'}</span>
                    </TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {!permissions.data || permissions.data.length === 0 ? (
                    <TableRow>
                      <TableCell colSpan={5} className="h-24 text-center">
                        {t('no_permissions_found') || 'No permissions found.'}
                      </TableCell>
                    </TableRow>
                  ) : (
                    permissions.data.map((permission) => (
                      <TableRow key={permission.id}>
                        <TableCell className="font-mono text-sm">{permission.name}</TableCell>
                        <TableCell>
                          <Badge variant="outline">{getGroupFromName(permission.name)}</Badge>
                        </TableCell>
                        <TableCell>
                          <Badge variant="secondary">{permission.name.split('.')[1] || '-'}</Badge>
                        </TableCell>
                        <TableCell>
                          <Badge variant="outline">{permission.guard_name}</Badge>
                        </TableCell>
                        <TableCell>
                          <DropdownMenu>
                            <DropdownMenuTrigger asChild>
                              <Button
                                aria-haspopup="true"
                                size="icon"
                                variant="ghost"
                              >
                                <MoreHorizontal className="h-4 w-4" />
                                <span className="sr-only">{t('toggle_menu') || 'Toggle menu'}</span>
                              </Button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="end">
                              <DropdownMenuLabel>{t('actions') || 'Actions'}</DropdownMenuLabel>
                              <DropdownMenuSeparator />
                              <DropdownMenuItem
                                onClick={() => navigate(pathFor('admin.permissions.edit', permission.id))}
                              >
                                <Edit className="mr-2 h-4 w-4" />
                                {t('edit') || 'Edit'}
                              </DropdownMenuItem>
                              <DropdownMenuSeparator />
                              <DropdownMenuItem
                                className="text-red-600"
                                onClick={() => handleDelete(permission)}
                              >
                                <Trash2 className="mr-2 h-4 w-4" />
                                {t('delete') || 'Delete'}
                              </DropdownMenuItem>
                            </DropdownMenuContent>
                          </DropdownMenu>
                        </TableCell>
                      </TableRow>
                    ))
                  )}
                </TableBody>
              </Table>
            </CardContent>
            <CardFooter className="flex flex-col sm:flex-row items-center justify-between gap-4">
              <div className="text-xs text-muted-foreground">
                {t('showing_permissions', { start: ((permissions.current_page - 1) * permissions.per_page) + 1, end: Math.min(permissions.current_page * permissions.per_page, permissions.total), total: permissions.total }) || `Showing ${((permissions.current_page - 1) * permissions.per_page) + 1}-${Math.min(permissions.current_page * permissions.per_page, permissions.total)} of ${permissions.total} permissions`}
              </div>

              {permissions.last_page > 1 && (
                <Pagination>
                  <PaginationContent>
                    <PaginationItem>
                      <PaginationPrevious
                        href="#"
                        onClick={(e) => {
                          e.preventDefault()
                          if (permissions.current_page > 1) handlePageChange(permissions.current_page - 1)
                        }}
                        className={permissions.current_page === 1 ? "pointer-events-none opacity-50" : ""}
                      />
                    </PaginationItem>

                    {generatePageNumbers().map((page, index) => (
                      <PaginationItem key={index}>
                        {page === '...' ? (
                          <span className="flex h-9 w-9 items-center justify-center text-sm">...</span>
                        ) : (
                          <PaginationLink
                            href="#"
                            onClick={(e) => {
                              e.preventDefault()
                              handlePageChange(page as number)
                            }}
                            isActive={page === permissions.current_page}
                          >
                            {page}
                          </PaginationLink>
                        )}
                      </PaginationItem>
                    ))}

                    <PaginationItem>
                      <PaginationNext
                        href="#"
                        onClick={(e) => {
                          e.preventDefault()
                          if (permissions.current_page < permissions.last_page) handlePageChange(permissions.current_page + 1)
                        }}
                        className={permissions.current_page === permissions.last_page ? "pointer-events-none opacity-50" : ""}
                      />
                    </PaginationItem>
                  </PaginationContent>
                </Pagination>
              )}
            </CardFooter>
          </Card>
        </div>
      </Main>

      <ConfirmDialog
        open={confirmDelete.open}
        onOpenChange={(open) => setConfirmDelete(prev => ({ ...prev, open }))}
        title={t('delete_permission') || 'Delete permission'}
        desc={t('are_you_sure_delete_permission') || 'Are you sure you want to delete this permission? This action cannot be undone.'}
        confirmText={t('delete') || 'Delete'}
        destructive
        handleConfirm={confirmDeletePermission}
      />
    </AuthenticatedLayout>
  )
}
