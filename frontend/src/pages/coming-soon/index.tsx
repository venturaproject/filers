import {IconPlanet} from '@tabler/icons-react'
import {GoBack} from "@/pages/errors/components/go-back"
import {AuthenticatedLayout} from "@/layouts"
import { useI18n } from '@/i18n/context'

export default function ComingSoon() {
  const { t } = useI18n()
  
  return (
    <AuthenticatedLayout title={t('coming_soon')} showHeader={false}>
      <div className='h-svh'>
        <div className='m-auto flex h-full w-full flex-col items-center justify-center gap-2'>
          <IconPlanet size={72}/>
          <h1 className='text-4xl font-bold leading-tight'>{t('coming_soon')}</h1>
          <p className='text-center text-muted-foreground'>
            {t('page_not_created_yet')} <br/>
            {t('stay_tuned')}
          </p>
          <GoBack/>
        </div>
      </div>
    </AuthenticatedLayout>
  )
}
