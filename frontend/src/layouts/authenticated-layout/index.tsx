import {AppSidebar} from "@/components/layout/app-sidebar"
import {Header} from '@/components/layout/header'
import {TopNav} from '@/components/layout/top-nav'
import {ProfileDropdown} from '@/components/profile-dropdown'
import {Search} from '@/components/search'
import NotificationButton from '@/components/notification/notification-button'

import {
  SidebarInset,
  SidebarProvider,
} from "@/components/ui/sidebar"
import {ThemeSwitch} from "@/components/theme-switch"
import { useEffect } from "react"
import { APP_NAME as appName } from "@/config/env"

export function AuthenticatedLayout({
    children,
    title,
    showHeader = true,
    withTopNav = true,
  }: any) {

  useEffect(() => {
    document.title = title ? `${title} - ${appName}` : appName
  }, [title])

  return (
    <>
      <SidebarProvider>
        <AppSidebar/>
        <SidebarInset>
          {showHeader && <Header>
            <div className='ml-auto flex items-center space-x-4'>
              <Search/>
              <NotificationButton/>
              <ThemeSwitch/>
              <ProfileDropdown/>
            </div>
          </Header>}

          {children}
        </SidebarInset>
      </SidebarProvider>
    </>
  )
}
