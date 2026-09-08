import {AppSidebar} from "@/components/layout/app-sidebar"
import {Header} from '@/components/layout/header'
import {TopNav} from '@/components/layout/top-nav'
import {ProfileDropdown} from '@/components/profile-dropdown'
import {Search} from '@/components/search'
import NotificationButton from '@/components/notification/notification-button';

import {
  SidebarInset,
  SidebarProvider,
} from "@/components/ui/sidebar"

import {ThemeSwitch} from "@/components/theme-switch"
import {Separator} from "@/components/ui/separator";
import SidebarNav from "./components/sidebar-nav";
import {Main} from "@/components/layout";
import {
  IconBrowserCheck,
  IconKey,
  IconNotification,
  IconPalette,
  IconUser
} from "@tabler/icons-react";
import { useI18n } from "@/i18n/context"

export function SettingLayout({children, title}: any) {
  const { t } = useI18n()

  const topNav = [
    {
      title: t('overview') || 'Overview',
      href: '/admin',
      isActive: true,
      disabled: false,
    },
    {
      title: t('customers') || 'Customers',
      href: '/admin/users',
      isActive: false,
      disabled: true,
    },
    {
      title: t('settings'),
      href: '/admin/settings',
      isActive: false,
      disabled: true,
    },
  ]

  const sidebarNavItems = [
    {
      title: t('profile'),
      icon: <IconUser size={18}/>,
      href: '/admin/settings',
    },
    {
      title: t('permissions'),
      icon: <IconKey size={18}/>,
      href: '/admin/settings/permissions',
    },
    {
      title: t('appearance'),
      icon: <IconPalette size={18}/>,
      href: '/admin/settings/appearance',
    },
    {
      title: t('notifications'),
      icon: <IconNotification size={18}/>,
      href: '/admin/settings/notifications',
    },
    {
      title: t('display'),
      icon: <IconBrowserCheck size={18}/>,
      href: '/admin/settings/display',
    },
  ]

  return (
    <>
      <SidebarProvider>
        <AppSidebar/>
        <SidebarInset>
          <Header>
            <TopNav links={topNav}/>
            <div className='ml-auto flex items-center space-x-4'>
              <Search/>
              <NotificationButton/>
              <ThemeSwitch/>
              <ProfileDropdown/>
            </div>
          </Header>

          <Main fixed>
            <div className='space-y-0.5'>
              <h1 className='text-2xl font-bold tracking-tight md:text-3xl'>
                {t('settings')}
              </h1>
              <p className='text-muted-foreground'>
                {t('settings_description')}
              </p>
            </div>
            <Separator className='my-4 lg:my-6'/>
            <div
              className='flex flex-1 flex-col space-y-2 md:space-y-2 overflow-hidden lg:flex-row lg:space-x-12 lg:space-y-0'>
              <aside className='top-0 lg:sticky lg:w-1/5'>
                <SidebarNav items={sidebarNavItems}/>
              </aside>
              <div className='flex w-full p-1 pr-4 overflow-y-hidden'>
                {children}
              </div>
            </div>
          </Main>
        </SidebarInset>
      </SidebarProvider>
    </>
  )
}
