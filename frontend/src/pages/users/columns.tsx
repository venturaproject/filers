import { createColumnHelper } from '@tanstack/react-table'
import { MoreHorizontal, Eye, Pencil, Trash2, ShieldAlert, Shield, User as UserIcon } from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { cn } from '@/lib/utils'
import { formatDistanceToNow } from 'date-fns'
import { es } from 'date-fns/locale'
import { callTypes } from './data/data'
import type { UserStatus } from './data/schema'

export interface User {
  id: string
  name: string
  firstName?: string
  lastName?: string
  username: string | null
  email: string
  status: UserStatus
  role: string
  avatar: string | null
  lastActivity: string | null
  createdAt: string
}

function getInitials(name: string): string {
  if (!name) return '?'
  return name
    .split(' ')
    .filter(Boolean)
    .slice(0, 2)
    .map((n) => n[0])
    .join('')
    .toUpperCase()
}

function UserAvatar({ user }: { user: User }) {
  return (
    <Avatar className="h-8 w-8 shrink-0">
      {user.avatar && <AvatarImage src={user.avatar} alt={user.name} />}
      <AvatarFallback className="bg-primary/10 text-primary text-xs font-semibold">
        {getInitials(user.name)}
      </AvatarFallback>
    </Avatar>
  )
}

const roleColors: Record<string, string> = {
  'Full Access': 'bg-purple-100 text-purple-700 border-purple-200',
  'Administrador': 'bg-blue-100 text-blue-700 border-blue-200',
  'Admin': 'bg-blue-100 text-blue-700 border-blue-200',
  'User': 'bg-gray-100 text-gray-700 border-gray-200',
}

const roleIcons: Record<string, React.ElementType> = {
  'Full Access': ShieldAlert,
  'Administrador': Shield,
  'Admin': Shield,
  'User': UserIcon,
}

const columnHelper = createColumnHelper<User>()

interface BuildColumnsOptions {
  t: (key: string, params?: Record<string, string | number>) => string
  selectedIds: Set<string>
  allPageSelected: boolean
  toggleSelectAll: (checked: boolean) => void
  toggleSelectRow: (id: string, checked: boolean) => void
  onView: (id: string) => void
  onEdit: (id: string) => void
  onDelete: (id: string) => void
}

export function buildUsersColumns({
  t,
  selectedIds,
  allPageSelected,
  toggleSelectAll,
  toggleSelectRow,
  onView,
  onEdit,
  onDelete,
}: BuildColumnsOptions) {
  return [
    columnHelper.display({
      id: 'select',
      enableHiding: false,
      header: () => (
        <Checkbox
          checked={allPageSelected}
          onCheckedChange={(c) => toggleSelectAll(!!c)}
          aria-label={t('select_all') || 'Seleccionar todo'}
        />
      ),
      cell: (info) => (
        <Checkbox
          checked={selectedIds.has(info.row.original.id)}
          onCheckedChange={(c) => toggleSelectRow(info.row.original.id, !!c)}
          aria-label={t('select_row') || 'Seleccionar fila'}
          onClick={(e) => e.stopPropagation()}
        />
      ),
    }),
    columnHelper.accessor('name', {
      id: 'name',
      header: () => t('col_nombre') || 'Nombre',
      cell: (info) => (
        <div className="flex items-center gap-3">
          <UserAvatar user={info.row.original} />
          <div className="min-w-0">
            <p className="font-medium text-sm truncate">{info.getValue()}</p>
            <p className="text-xs text-muted-foreground truncate">
              {info.row.original.email}
            </p>
          </div>
        </div>
      ),
    }),
    columnHelper.accessor('username', {
      id: 'username',
      header: () => t('username') || 'Usuario',
      cell: (info) => (
        <span className="font-mono text-sm text-muted-foreground">
          {info.getValue() ?? '-'}
        </span>
      ),
    }),
    columnHelper.accessor('status', {
      id: 'status',
      header: () => t('col_estado') || 'Estado',
      cell: (info) => {
        const status = info.getValue()
        const cls = callTypes.get(status) ?? ''
        const labels: Record<string, string> = {
          active: t('status_active') || 'Activo',
          inactive: t('status_inactive') || 'Inactivo',
          suspended: t('status_suspended') || 'Suspendido',
        }
        return (
          <Badge variant="outline" className={cn('rounded-full capitalize', cls)}>
            {labels[status] ?? status}
          </Badge>
        )
      },
    }),
    columnHelper.accessor('role', {
      id: 'role',
      header: () => t('role') || 'Rol',
      cell: (info) => {
        const role = info.getValue()
        if (!role) return <span className="text-sm text-muted-foreground">-</span>
        const colorClass = roleColors[role] ?? 'bg-gray-100 text-gray-700 border-gray-200'
        const IconComponent = roleIcons[role] ?? UserIcon
        return (
          <Badge variant="outline" className={`gap-1.5 font-medium ${colorClass}`}>
            <IconComponent className="h-3.5 w-3.5" />
            <span>{role}</span>
          </Badge>
        )
      },
    }),
    columnHelper.accessor('lastActivity', {
      id: 'lastActivity',
      header: () => t('col_last_activity') || 'Última actividad',
      cell: (info) => {
        const val = info.getValue()
        if (!val) return <span className="text-xs text-muted-foreground">—</span>
        try {
          return (
            <span className="text-xs text-muted-foreground">
              {formatDistanceToNow(new Date(val), { addSuffix: true, locale: es })}
            </span>
          )
        } catch {
          return <span className="text-xs text-muted-foreground">—</span>
        }
      },
    }),
    columnHelper.display({
      id: 'actions',
      enableHiding: false,
      header: () => <span className="sr-only">{t('actions') || 'Acciones'}</span>,
      cell: (info) => (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button aria-haspopup="true" size="icon" variant="ghost">
              <MoreHorizontal className="h-4 w-4" />
              <span className="sr-only">{t('actions') || 'Acciones'}</span>
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuLabel>{t('actions') || 'Acciones'}</DropdownMenuLabel>
            <DropdownMenuSeparator />
            <DropdownMenuItem
              onClick={() => onView(info.row.original.id)}
            >
              <Eye className="mr-2 h-4 w-4" />
              {t('view') || 'Ver'}
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={() => onEdit(info.row.original.id)}
            >
              <Pencil className="mr-2 h-4 w-4" />
              {t('edit') || 'Editar'}
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem
              className="text-destructive focus:text-destructive"
              onClick={() => onDelete(info.row.original.id)}
            >
              <Trash2 className="mr-2 h-4 w-4" />
              {t('delete') || 'Eliminar'}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      ),
    }),
  ]
}

export const userColumnLabels = (t: (k: string) => string): Record<string, string> => ({
  name: t('col_nombre') || 'Nombre',
  username: t('username') || 'Usuario',
  status: t('col_estado') || 'Estado',
  role: t('role') || 'Rol',
  lastActivity: t('col_last_activity') || 'Última actividad',
})
