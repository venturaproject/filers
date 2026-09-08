import { useState } from 'react'
import { ChevronDownIcon } from '@radix-ui/react-icons'
import { cn } from '@/lib/utils'
import { Button, buttonVariants } from '@/components/ui/button'
import { Label } from '@/components/ui/label'
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group'
import { useI18n } from '@/i18n/context'
import { useTheme } from '@/context/theme-context'
import { axios } from '@/lib/axios'
import { endpoints } from '@/lib/endpoints'
import { toast } from 'sonner'

export function AppearanceForm() {
  const { t } = useI18n()
  const { theme, setTheme } = useTheme()

  const [font, setFont] = useState<string>(() => localStorage.getItem('font') ?? 'inter')
  const [saving, setSaving] = useState(false)

  function handleThemeChange(value: 'light' | 'dark' | 'system') {
    setTheme(value)
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    setSaving(true)
    localStorage.setItem('font', font)
    try {
      await axios.put(endpoints.settings.appearance, { theme, font })
      toast.success(t('appearance_saved'))
    } catch {
      toast.error(t('something_went_wrong'))
    } finally {
      setSaving(false)
    }
  }

  return (
    <form onSubmit={handleSubmit} className='space-y-8'>
      {/* Font */}
      <div className='space-y-2'>
        <Label>{t('font')}</Label>
        <div className='relative w-max'>
          <select
            value={font}
            onChange={(e) => setFont(e.target.value)}
            className={cn(buttonVariants({ variant: 'outline' }), 'w-[200px] appearance-none font-normal')}
          >
            <option value='inter'>Inter</option>
            <option value='manrope'>Manrope</option>
            <option value='system'>{t('system')}</option>
          </select>
          <ChevronDownIcon className='absolute right-3 top-2.5 h-4 w-4 opacity-50' />
        </div>
        <p className='text-sm text-muted-foreground'>{t('font_description')}</p>
      </div>

      {/* Theme */}
      <div className='space-y-2'>
        <Label>{t('theme')}</Label>
        <p className='text-sm text-muted-foreground'>{t('theme_description')}</p>
        <RadioGroup
          value={theme}
          onValueChange={handleThemeChange}
          className='grid max-w-lg grid-cols-3 gap-4 pt-2'
        >
          {/* Light */}
          <Label className='[&:has([data-state=checked])>div]:border-primary cursor-pointer'>
            <RadioGroupItem value='light' className='sr-only' />
            <div className='items-center rounded-md border-2 border-muted p-1 hover:border-accent'>
              <div className='space-y-2 rounded-sm bg-[#ecedef] p-2'>
                <div className='space-y-2 rounded-md bg-white p-2 shadow-sm'>
                  <div className='h-2 w-[80px] rounded-lg bg-[#ecedef]' />
                  <div className='h-2 w-[100px] rounded-lg bg-[#ecedef]' />
                </div>
                <div className='flex items-center space-x-2 rounded-md bg-white p-2 shadow-sm'>
                  <div className='h-4 w-4 rounded-full bg-[#ecedef]' />
                  <div className='h-2 w-[100px] rounded-lg bg-[#ecedef]' />
                </div>
              </div>
            </div>
            <span className='block w-full p-2 text-center text-sm font-normal'>{t('light')}</span>
          </Label>

          {/* Dark */}
          <Label className='[&:has([data-state=checked])>div]:border-primary cursor-pointer'>
            <RadioGroupItem value='dark' className='sr-only' />
            <div className='items-center rounded-md border-2 border-muted bg-popover p-1 hover:bg-accent hover:text-accent-foreground'>
              <div className='space-y-2 rounded-sm bg-slate-950 p-2'>
                <div className='space-y-2 rounded-md bg-slate-800 p-2 shadow-sm'>
                  <div className='h-2 w-[80px] rounded-lg bg-slate-400' />
                  <div className='h-2 w-[100px] rounded-lg bg-slate-400' />
                </div>
                <div className='flex items-center space-x-2 rounded-md bg-slate-800 p-2 shadow-sm'>
                  <div className='h-4 w-4 rounded-full bg-slate-400' />
                  <div className='h-2 w-[100px] rounded-lg bg-slate-400' />
                </div>
              </div>
            </div>
            <span className='block w-full p-2 text-center text-sm font-normal'>{t('dark')}</span>
          </Label>

          {/* System */}
          <Label className='[&:has([data-state=checked])>div]:border-primary cursor-pointer'>
            <RadioGroupItem value='system' className='sr-only' />
            <div className='items-center rounded-md border-2 border-muted p-1 hover:border-accent'>
              <div className='space-y-2 rounded-sm bg-[#ecedef] p-2'>
                <div className='space-y-2 rounded-md bg-gradient-to-r from-white to-slate-800 p-2 shadow-sm'>
                  <div className='h-2 w-[80px] rounded-lg bg-slate-400' />
                  <div className='h-2 w-[100px] rounded-lg bg-slate-400' />
                </div>
                <div className='flex items-center space-x-2 rounded-md bg-gradient-to-r from-white to-slate-800 p-2 shadow-sm'>
                  <div className='h-4 w-4 rounded-full bg-slate-400' />
                  <div className='h-2 w-[100px] rounded-lg bg-slate-400' />
                </div>
              </div>
            </div>
            <span className='block w-full p-2 text-center text-sm font-normal'>{t('system')}</span>
          </Label>
        </RadioGroup>
      </div>

      <Button type='submit' disabled={saving}>
        {saving ? t('saving') : t('update_preferences')}
      </Button>
    </form>
  )
}
