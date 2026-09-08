import {GoBack} from "./components/go-back"
import { DocumentTitle } from '@/components/document-title'

export default function NotFoundError() {
  return (
    <>
      <DocumentTitle title='Página no encontrada'/>
      <div className='h-svh'>
        <div className='m-auto flex h-full w-full flex-col items-center justify-center gap-2'>
          <h1 className='text-[7rem] font-bold leading-tight'>404</h1>
          <span className='font-medium'>Página no encontrada</span>
          <p className='text-center text-muted-foreground'>
            La página que buscas no existe <br/>
            o ha sido eliminada.
          </p>
          <GoBack/>
        </div>
      </div>
    </>
  )
}
