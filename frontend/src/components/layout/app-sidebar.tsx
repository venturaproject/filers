import { useMemo } from 'react'
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarHeader,
  SidebarRail,
} from '@/components/ui/sidebar'
import { NavGroup } from '@/components/layout/nav-group'
import { TeamSwitcher } from '@/components/layout/team-switcher'
import { SidebarUserMenu } from '@/components/layout/sidebar-user-menu'
import { DynamicSidebarData } from './data/dynamic-sidebar-data'
import { usePermission } from '@/hooks/use-permission'

export function AppSidebar({ ...props }: React.ComponentProps<typeof Sidebar>) {
  const { filterNavGroups } = usePermission()
  const sidebarData = DynamicSidebarData()

  const filteredNavGroups = useMemo(
    () => filterNavGroups(sidebarData.navGroups),
    [filterNavGroups]
  )

  return (
    <Sidebar collapsible='icon' variant='floating' {...props}>
      <SidebarHeader>
        <TeamSwitcher teams={sidebarData.teams} />
      </SidebarHeader>
      <SidebarContent>
        {filteredNavGroups.map((props) => (
          <NavGroup key={props.title} {...props} />
        ))}
      </SidebarContent>
      <SidebarFooter>
        <SidebarUserMenu />
      </SidebarFooter>
      <SidebarRail />
    </Sidebar>
  )
}
