import * as React from 'react'
import {
  SidebarMenu,
  SidebarMenuItem,
  useSidebar,
} from '@/components/ui/sidebar'

export function TeamSwitcher({
  teams,
}: {
  teams: {
    name: string
    logo: React.ElementType
    plan: string
  }[]
}) {
  const { isMobile, state } = useSidebar();
  const activeTeam = teams[0]

  return (
    <SidebarMenu>
      <SidebarMenuItem className={`px-2 py-3 ${state === 'collapsed' ? 'flex justify-center' : ''}`}>
        {React.createElement(activeTeam.logo, {
          className: state === 'collapsed' ? 'h-12 w-auto' : 'h-10 w-auto max-w-full',
          isCollapsed: state === 'collapsed' || isMobile
        })}
      </SidebarMenuItem>
    </SidebarMenu>
  )
}
