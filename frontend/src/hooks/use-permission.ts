import { useAuthStore } from '@/lib/auth'
import { FULL_ACCESS_ROLES } from '@/config/env'
import { type NavGroup, type NavItem } from '@/components/layout/types'

export function usePermission() {
  const { permissions, roles } = useAuthStore()

  const can = (permission: string): boolean => permissions.includes(permission)
  const canAny = (ps: string[]): boolean => ps.some(p => permissions.includes(p))
  const canAll = (ps: string[]): boolean => ps.every(p => permissions.includes(p))
  const hasRole = (role: string): boolean => roles.includes(role)
  const hasAnyRole = (rs: string[]): boolean => rs.some(r => roles.includes(r))
  const hasFullAccess = (): boolean => roles.some(r => FULL_ACCESS_ROLES.includes(r))

  const checkPermission = (permission?: string | string[]): boolean => {
    if (!permission) return true
    if (hasFullAccess()) return true
    if (Array.isArray(permission)) return permission.some(p => can(p))
    return can(permission)
  }

  const filterNavItem = (item: NavItem): NavItem | null => {
    if ('items' in item && item.items) {
      const filteredItems = item.items.filter(subItem => checkPermission(subItem.permission))
      if (filteredItems.length === 0) return null
      return { ...item, items: filteredItems }
    }
    if (!checkPermission(item.permission)) return null
    return item
  }

  const filterNavGroups = (groups: NavGroup[]): NavGroup[] => {
    return groups
      .map(group => ({
        ...group,
        items: group.items.map(filterNavItem).filter((item): item is NavItem => item !== null),
      }))
      .filter(group => group.items.length > 0)
  }

  return { can, canAny, canAll, hasRole, hasAnyRole, hasFullAccess, checkPermission, filterNavGroups, permissions, roles }
}
