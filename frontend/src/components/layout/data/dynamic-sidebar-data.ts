import {
  IconBrowserCheck,
  IconChecklist,
  IconLayoutDashboard,
  IconNotification,
  IconPalette,
  IconSettings,
  IconUpload,
  IconUserCog,
  IconUsers,
} from '@tabler/icons-react'
import { type SidebarData } from '../types'
import FilersLogo from '../filers-logo'
import { useI18n } from '@/i18n/context'

export const DynamicSidebarData = () => {
  const { t } = useI18n()

  const sidebarData: SidebarData = {
    teams: [
      {
        name: 'Filers',
        logo: FilersLogo,
        plan: 'File Processing API',
      },
    ],
    navGroups: [
      {
        title: t('general'),
        items: [
          {
            title: t('dashboard'),
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
            title: 'Procesamientos',
            url: '/admin/jobs',
            icon: IconChecklist,
            permission: 'files.batch',
          },
        ],
      },
      {
        title: t('access_control'),
        items: [
          {
            // Roles and Permissions live as tabs inside this page
            // (see AccessControlTabs), not as separate nav entries.
            title: t('users'),
            url: '/admin/users',
            icon: IconUsers,
            permission: ['users.view', 'roles.view', 'permissions.view'],
          },
        ],
      },
      {
        title: t('others'),
        items: [
          {
            title: t('configuration'),
            icon: IconSettings,
            items: [
              {
                title: t('profile'),
                url: '/admin/settings',
                icon: IconUserCog,
              },
              {
                title: t('appearance'),
                url: '/admin/settings/appearance',
                icon: IconPalette,
              },
              {
                title: t('notifications'),
                url: '/admin/settings/notifications',
                icon: IconNotification,
              },
              {
                title: t('display'),
                url: '/admin/settings/display',
                icon: IconBrowserCheck,
              },
            ],
          },
        ],
      },
    ],
  };

  return sidebarData;
};