import {
  IconChecklist,
  IconGitCompare,
  IconRoute,
  IconLayoutDashboard,
  IconSettings,
  IconUserCog,
  IconUsers,
  IconUpload,
  IconPalette,
  IconBrowserCheck,
  IconNotification,
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
          title: 'Banco de trabajo',
          url: '/admin/process',
          icon: IconUpload,
          permission: 'files.process',
        },
        {
          title: 'Comparar',
          url: '/admin/compare',
          icon: IconGitCompare,
          permission: 'files.process',
        },
        {
          title: 'Pipeline',
          url: '/admin/pipeline',
          icon: IconRoute,
          permission: 'files.process',
        },
        {
          title: 'Procesamientos',
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
          // Roles/Permisos are tabs within this page (AccessControlTabs).
          title: 'Usuarios',
          url: '/admin/users',
          icon: IconUsers,
          permission: ['users.view', 'roles.view', 'permissions.view'],
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
