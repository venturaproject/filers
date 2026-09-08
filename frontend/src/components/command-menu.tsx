import React, { useState, useEffect, useMemo } from 'react'
import {
  IconArrowRightDashed,
  IconDeviceLaptop,
  IconMoon,
  IconSearch,
  IconSun,
} from '@tabler/icons-react'
import { useSearch } from '@/context/search-context'
import { useTheme } from '@/context/theme-context'
import { useI18n } from '@/i18n/context'
import { router } from '@/lib/router-singleton'
import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
} from '@/components/ui/command'
import { DialogTitle, DialogDescription } from '@/components/ui/dialog'
import { sidebarData } from './layout/data/sidebar-data'
import { ScrollArea } from './ui/scroll-area'

interface SearchResults {
  users: Array<{
    id: number;
    name: string;
    email: string;
  }>;
  roles: Array<{
    id: number;
    name: string;
  }>;
  permissions: Array<{
    id: number;
    name: string;
    guard_name: string;
  }>;
  categories: Array<{
    id: number;
    name: string;
  }>;
  tags: Array<{
    id: number;
    name: string;
  }>;
}

export function CommandMenu() {
  const { t } = useI18n()
  const { setTheme } = useTheme()
  const { open, setOpen } = useSearch()
  const [searchQuery, setSearchQuery] = useState('')
  const [searchResults, setSearchResults] = useState<SearchResults | null>(null)
  const [isLoading, setIsLoading] = useState(false)

  const runCommand = React.useCallback(
    (command: () => unknown) => {
      setOpen(false)
      command()
    },
    [setOpen]
  )

  // Handle search input changes
  useEffect(() => {
    if (searchQuery.trim() === '') {
      setSearchResults(null)
      return
    }

    setIsLoading(true)

    // Debounce the search request
    const timeoutId = setTimeout(() => {
      fetch(`/admin/search?q=${encodeURIComponent(searchQuery)}`)
        .then(response => {
          if (response.ok) {
            return response.json();
          } else {
            throw new Error(`HTTP error! status: ${response.status}`);
          }
        })
        .then(data => {
          setSearchResults(data)
          setIsLoading(false)
        })
        .catch(error => {
          console.error('Search error:', error)
          setIsLoading(false)
        })
    }, 300)

    return () => clearTimeout(timeoutId)
  }, [searchQuery])

  // Determine if we're showing search results or navigation
  const showSearchResults = searchQuery.trim() !== ''

  return (
    <CommandDialog modal open={open} onOpenChange={setOpen}>
      <DialogTitle className="sr-only">{t('command_menu_title')}</DialogTitle>
      <DialogDescription className="sr-only">{t('command_menu_description')}</DialogDescription>
      <CommandInput
        placeholder={t('command_menu_placeholder')}
        value={searchQuery}
        onValueChange={setSearchQuery}
      />
      <CommandList>
        <ScrollArea type='hover' className='h-72 pr-1'>
          {showSearchResults ? (
            <>
              {isLoading ? (
                <CommandItem disabled>
                  <IconSearch className="mr-2 h-4 w-4 animate-spin" />
                  {t('searching')}
                </CommandItem>
              ) : searchResults ? (
                <>
                  {searchResults.users.length > 0 && (
                    <CommandGroup heading={t('command_users')}>
                      {searchResults.users.map((user) => (
                        <CommandItem
                          key={`user-${user.id}`}
                          value={user.name}
                          onSelect={() => {
                            runCommand(() => router.visit(`/admin/users/${user.id}/edit`))
                          }}
                        >
                          <div className='mr-2 flex h-4 w-4 items-center justify-center'>
                            <IconArrowRightDashed className='size-2 text-muted-foreground/80' />
                          </div>
                          {user.name} <span className="ml-2 text-xs text-muted-foreground">({user.email})</span>
                        </CommandItem>
                      ))}
                    </CommandGroup>
                  )}

                  {searchResults.categories.length > 0 && (
                    <CommandGroup heading={t('command_categories')}>
                      {searchResults.categories.map((category) => (
                        <CommandItem
                          key={`category-${category.id}`}
                          value={category.name}
                          onSelect={() => {
                            runCommand(() => router.visit(`/admin/categories/${category.id}/edit`))
                          }}
                        >
                          <div className='mr-2 flex h-4 w-4 items-center justify-center'>
                            <IconArrowRightDashed className='size-2 text-muted-foreground/80' />
                          </div>
                          {category.name}
                        </CommandItem>
                      ))}
                    </CommandGroup>
                  )}

                  {searchResults.tags.length > 0 && (
                    <CommandGroup heading={t('command_tags')}>
                      {searchResults.tags.map((tag) => (
                        <CommandItem
                          key={`tag-${tag.id}`}
                          value={tag.name}
                          onSelect={() => {
                            runCommand(() => router.visit(`/admin/tags/${tag.id}/edit`))
                          }}
                        >
                          <div className='mr-2 flex h-4 w-4 items-center justify-center'>
                            <IconArrowRightDashed className='size-2 text-muted-foreground/80' />
                          </div>
                          {tag.name}
                        </CommandItem>
                      ))}
                    </CommandGroup>
                  )}

                  {searchResults.roles.length > 0 && (
                    <CommandGroup heading={t('command_roles')}>
                      {searchResults.roles.map((role) => (
                        <CommandItem
                          key={`role-${role.id}`}
                          value={role.name}
                          onSelect={() => {
                            runCommand(() => router.visit(`/admin/roles/${role.id}/edit`))
                          }}
                        >
                          <div className='mr-2 flex h-4 w-4 items-center justify-center'>
                            <IconArrowRightDashed className='size-2 text-muted-foreground/80' />
                          </div>
                          {role.name}
                        </CommandItem>
                      ))}
                    </CommandGroup>
                  )}

                  {searchResults.permissions.length > 0 && (
                    <CommandGroup heading={t('command_permissions')}>
                      {searchResults.permissions.map((permission) => (
                        <CommandItem
                          key={`permission-${permission.id}`}
                          value={permission.name}
                          onSelect={() => {
                            runCommand(() => router.visit(`/admin/permissions/${permission.id}/edit`))
                          }}
                        >
                          <div className='mr-2 flex h-4 w-4 items-center justify-center'>
                            <IconArrowRightDashed className='size-2 text-muted-foreground/80' />
                          </div>
                          {permission.name}
                        </CommandItem>
                      ))}
                    </CommandGroup>
                  )}

                  {Object.values(searchResults).every(arr => arr.length === 0) && (
                    <CommandEmpty>{t('command_no_results')}</CommandEmpty>
                  )}
                </>
              ) : (
                <CommandEmpty>{t('command_no_results')}</CommandEmpty>
              )}
            </>
          ) : (
            <>
              <CommandEmpty>{t('command_no_results')}</CommandEmpty>
              {sidebarData.navGroups.map((group) => (
                <CommandGroup key={group.title} heading={group.title}>
                  {group.items.map((navItem, i) => {
                    if (navItem.url)
                      return (
                        <CommandItem
                          key={`${navItem.url}-${i}`}
                          value={navItem.title}
                          onSelect={() => {
                            runCommand(() => router.visit(navItem.url))
                          }}
                        >
                          <div className='mr-2 flex h-4 w-4 items-center justify-center'>
                            <IconArrowRightDashed className='size-2 text-muted-foreground/80' />
                          </div>
                          {navItem.title}
                        </CommandItem>
                      )

                    return navItem.items?.map((subItem, i) => (
                      <CommandItem
                        key={`${subItem.url}-${i}`}
                        value={subItem.title}
                        onSelect={() => {
                          runCommand(() => router.visit(subItem.url))
                        }}
                      >
                        <div className='mr-2 flex h-4 w-4 items-center justify-center'>
                          <IconArrowRightDashed className='size-2 text-muted-foreground/80' />
                        </div>
                        {subItem.title}
                      </CommandItem>
                    ))
                  })}
                </CommandGroup>
              ))}
              <CommandSeparator />
              <CommandGroup heading={t('command_theme')}>
                <CommandItem onSelect={() => runCommand(() => setTheme('light'))}>
                  <IconSun /> <span>{t('command_theme_light')}</span>
                </CommandItem>
                <CommandItem onSelect={() => runCommand(() => setTheme('dark'))}>
                  <IconMoon className='scale-90' />
                  <span>{t('command_theme_dark')}</span>
                </CommandItem>
                <CommandItem onSelect={() => runCommand(() => setTheme('system'))}>
                  <IconDeviceLaptop />
                  <span>{t('command_theme_system')}</span>
                </CommandItem>
              </CommandGroup>
            </>
          )}
        </ScrollArea>
      </CommandList>
    </CommandDialog>
  )
}
