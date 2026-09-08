import { SidebarMenu, SidebarMenuItem } from '@/components/ui/sidebar';
import { APP_NAME } from '@/config/env';

export function CopyrightFooter() {
  const appName = APP_NAME;
  const currentYear = new Date().getFullYear();

  return (
    <SidebarMenu>
      <SidebarMenuItem className="justify-center text-center py-2">
        <div className="text-xs text-muted-foreground">
          {appName} © {currentYear}
        </div>
      </SidebarMenuItem>
    </SidebarMenu>
  );
}