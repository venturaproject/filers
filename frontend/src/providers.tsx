import * as React from "react"
import {QueryClientProvider} from "@tanstack/react-query"
import {HelmetProvider} from "react-helmet-async"
import {Toaster} from "@/components/ui/toaster"
import {Toaster as SonnerToaster} from "sonner"
import {TooltipProvider} from "@/components/ui/tooltip"
import {NuqsAdapter} from "@/lib/nuqs"
import {queryClient} from "@/lib/react-query"
import {ThemeProvider} from "@/context/theme-context"
import {SearchProvider} from "@/context/search-context"
import {I18nProvider} from "@/i18n/context"

export function Providers({children}: {children: React.ReactNode}) {
  return (
    <HelmetProvider>
      <NuqsAdapter>
        <QueryClientProvider client={queryClient}>
          <I18nProvider>
            <SearchProvider>
              <ThemeProvider
                defaultTheme="light"
                storageKey="app-ui-theme"
              >
                <TooltipProvider>{children}</TooltipProvider>

                <Toaster/>
                <SonnerToaster position="top-right" />
              </ThemeProvider>
            </SearchProvider>
          </I18nProvider>

          {/* Devtools — only in development */}
          {import.meta.env.DEV && (
            <React.Suspense fallback={null}>
              <DevTools />
            </React.Suspense>
          )}
        </QueryClientProvider>
      </NuqsAdapter>
    </HelmetProvider>
  )
}

const DevTools = React.lazy(() =>
  import("@tanstack/react-query-devtools").then((m) => ({
    default: () => <m.ReactQueryDevtools buttonPosition="bottom-right" />,
  }))
)
