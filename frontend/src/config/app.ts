import { FULL_ACCESS_ROLES } from './env'

export interface AppConfig {
  fullAccessRoles: string[]
  guardName: string
  defaultPerPage: number
}

export const appConfig: AppConfig = {
  fullAccessRoles: FULL_ACCESS_ROLES,
  guardName: 'api',
  defaultPerPage: 20,
}

export const isFullAccess = (roleNames: string[]): boolean => {
  return roleNames.some((role) => appConfig.fullAccessRoles.includes(role))
}

export const hasPermission = (userPermissions: string[], requiredPermission: string): boolean => {
  return userPermissions.includes(requiredPermission)
}

export const hasAnyPermission = (
  userPermissions: string[],
  requiredPermissions: string[]
): boolean => {
  return requiredPermissions.some((perm) => userPermissions.includes(perm))
}

export const hasAllPermissions = (
  userPermissions: string[],
  requiredPermissions: string[]
): boolean => {
  return requiredPermissions.every((perm) => userPermissions.includes(perm))
}