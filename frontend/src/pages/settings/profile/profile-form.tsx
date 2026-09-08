import { useRef, useState } from 'react'
import { useI18n } from '@/i18n/context'
import { toast } from 'sonner'
import { useAuthStore } from '@/lib/auth'
import { axios } from '@/lib/axios'
import { endpoints } from '@/lib/endpoints'
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar'
import { Badge } from '@/components/ui/badge'
import { IconCamera, IconMail, IconUser, IconShield } from '@tabler/icons-react'

export default function ProfileForm() {
  const { t } = useI18n()
  const user = useAuthStore((s) => s.user)
  const permissions = useAuthStore((s) => s.permissions)
  const roles = useAuthStore((s) => s.roles)
  const setAuth = useAuthStore((s) => s.setAuth)

  const fileInputRef = useRef<HTMLInputElement>(null)
  const [preview, setPreview] = useState<string | null>(user?.avatar ?? null)
  const [uploading, setUploading] = useState(false)

  const initials = (user?.name ?? 'User')
    .split(' ')
    .map((n) => n[0])
    .join('')
    .toUpperCase()
    .slice(0, 2)

  async function handleFileChange(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0]
    if (!file) return

    setPreview(URL.createObjectURL(file))
    setUploading(true)

    const formData = new FormData()
    formData.append('avatar', file)

    try {
      const response = await axios.post(endpoints.auth.avatar, formData, {
        headers: { 'Content-Type': 'multipart/form-data' },
      })
      const updatedUser = response.data?.user
      if (updatedUser) {
        setAuth(updatedUser, permissions, roles)
        setPreview(updatedUser.avatar ?? null)
      }
      toast.success(t('avatar_updated'))
    } catch {
      setPreview(user?.avatar ?? null)
      toast.error(t('something_went_wrong'))
    } finally {
      setUploading(false)
    }
  }

  if (!user) return <div>Loading...</div>

  return (
    <div className='space-y-8'>
      {/* Avatar */}
      <div className='flex items-center gap-6'>
        <div className='relative'>
          <Avatar className='h-20 w-20'>
            <AvatarImage src={preview ?? undefined} alt={user.name ?? 'User'} />
            <AvatarFallback className='text-lg'>{initials}</AvatarFallback>
          </Avatar>
          <button
            type='button'
            onClick={() => fileInputRef.current?.click()}
            disabled={uploading}
            className='absolute -bottom-1 -right-1 flex h-7 w-7 items-center justify-center rounded-full bg-primary text-primary-foreground shadow hover:bg-primary/90 disabled:opacity-50'
          >
            <IconCamera size={14} />
          </button>
          <input
            ref={fileInputRef}
            type='file'
            accept='image/*'
            className='hidden'
            onChange={handleFileChange}
          />
        </div>
        <div>
          <p className='font-medium'>{user.name}</p>
          <p className='text-sm text-muted-foreground'>{t('click_camera_to_change_avatar')}</p>
        </div>
      </div>

      {/* Read-only fields */}
      <div className='space-y-4'>
        <div className='flex items-center gap-3 rounded-md border bg-muted/40 px-4 py-3'>
          <IconUser size={16} className='shrink-0 text-muted-foreground' />
          <div>
            <p className='text-xs text-muted-foreground'>{t('name')}</p>
            <p className='text-sm font-medium'>{user.name}</p>
          </div>
        </div>

        <div className='flex items-center gap-3 rounded-md border bg-muted/40 px-4 py-3'>
          <IconMail size={16} className='shrink-0 text-muted-foreground' />
          <div>
            <p className='text-xs text-muted-foreground'>{t('email')}</p>
            <p className='text-sm font-medium'>{user.email}</p>
          </div>
        </div>

        <div className='flex items-center gap-3 rounded-md border bg-muted/40 px-4 py-3'>
          <IconShield size={16} className='shrink-0 text-muted-foreground' />
          <div>
            <p className='text-xs text-muted-foreground'>{t('roles')}</p>
            <div className='mt-1 flex flex-wrap gap-1'>
              {(user.roles ?? []).map((role) => (
                <Badge key={role} variant='secondary'>{role}</Badge>
              ))}
            </div>
          </div>
        </div>
      </div>

      <p className='text-xs text-muted-foreground'>{t('profile_managed_by_admin')}</p>
    </div>
  )
}
