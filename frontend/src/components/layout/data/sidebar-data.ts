import {
  IconBarrierBlock,
  IconBug,
  IconChecklist,
  IconError404,
  IconLayoutDashboard,
  IconLock,
  IconLockAccess,
  IconServerOff,
  IconSettings,
  IconUserCog,
  IconUsers,
  IconUpload,
  IconShieldLock,
  IconPalette,
  IconBrowserCheck,
  IconNotification,
  IconTool,
} from '@tabler/icons-react'
import { type SidebarData } from '../types'
import FilersLogo from '../filers-logo'

export const sidebarData: SidebarData = {
  teams: [
    {
      name: 'Filers',
      logo: FilersLogo,
      plan: 'File Processing API',
    },
  ],
  navGroups: [
    {
      title: 'General',
      items: [
        {
          title: 'Dashboard',
          url: '/admin',
          icon: IconLayoutDashboard,
        },
      ],
    },
    {
      title: 'Procesamiento',
      items: [
        {
          title: 'Procesar archivo',
          url: '/admin/process',
          icon: IconUpload,
          permission: 'files.process',
        },
        {
          title: 'Trabajos batch',
          url: '/admin/jobs',
          icon: IconChecklist,
          permission: 'files.batch',
        },
      ],
    },
    {
      title: 'Control de Acceso',
      items: [
        {
          title: 'Usuarios',
          url: '/admin/users',
          icon: IconUsers,
          permission: 'users.view',
        },
        {
          title: 'Roles',
          url: '/admin/roles',
          icon: IconShieldLock,
          permission: 'roles.view',
        },
        {
          title: 'Permisos',
          url: '/admin/permissions',
          icon: IconLockAccess,
          permission: 'permissions.view',
        },
      ],
    },
    {
      title: 'Otros',
      items: [
        {
          title: 'Configuración',
          icon: IconSettings,
          items: [
            {
              title: 'Perfil',
              url: '/admin/settings',
              icon: IconUserCog,
            },
            {
              title: 'Apariencia',
              url: '/admin/settings/appearance',
              icon: IconPalette,
            },
            {
              title: 'Notificaciones',
              url: '/admin/settings/notifications',
              icon: IconNotification,
            },
            {
              title: 'Visualización',
              url: '/admin/settings/display',
              icon: IconBrowserCheck,
            },
          ],
        },
      ],
    },
  ],
}
