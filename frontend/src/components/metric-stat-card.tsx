import { Card, CardContent } from '@/components/ui/card'
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip'
import { Info, type LucideIcon } from 'lucide-react'

interface MetricStatCardProps {
  title: string
  value: number | string
  subtitle: string
  icon: LucideIcon
  tooltip?: string
  trend?: number
  trendLabel?: string
  detailsLabel?: string
  sparklineColor: string
  sparklineData?: number[]
}

function buildSparklinePath(values: number[]): string {
  const points = values.length > 1 ? values : [4, 7, 5, 8, 6, 9]
  const max = Math.max(...points)
  const min = Math.min(...points)
  const range = max - min || 1
  const coordinates = points.map((value, index) => ({
    x: (index / Math.max(points.length - 1, 1)) * 100,
    y: 34 - ((value - min) / range) * 28 - 3,
  }))

  return coordinates
    .map((point, index) => {
      if (index === 0) {
        return `M ${point.x.toFixed(2)} ${point.y.toFixed(2)}`
      }

      const previous = coordinates[index - 1]
      const controlX = previous.x + (point.x - previous.x) * 0.35

      return `C ${controlX.toFixed(2)} ${previous.y.toFixed(2)}, ${controlX.toFixed(2)} ${point.y.toFixed(2)}, ${point.x.toFixed(2)} ${point.y.toFixed(2)}`
    })
    .join(' ')
}

function formatMetricValue(value: number | string): string {
  return typeof value === 'number' ? value.toLocaleString('es-ES') : value
}

export function MetricStatCard({
  title,
  value,
  subtitle,
  icon: Icon,
  tooltip,
  sparklineColor,
  sparklineData = [5, 8, 6, 10, 7, 11, 9, 12],
}: MetricStatCardProps) {
  return (
    <Card className="rounded-[14px] border border-border bg-card shadow-sm">
      <CardContent className="p-5">
        <div className="flex items-center justify-between gap-4">
          <div className="flex items-center gap-2 text-sm font-semibold text-card-foreground">
            <Icon className="h-4 w-4 text-foreground" />
            <span>{title}</span>
          </div>

          <TooltipProvider delayDuration={150}>
            <Tooltip>
              <TooltipTrigger asChild>
                <button
                  type="button"
                  aria-label={tooltip ?? title}
                  className="flex h-6 w-6 items-center justify-center rounded-full border border-border text-muted-foreground transition-colors hover:border-input hover:text-foreground"
                >
                  <Info className="h-3.5 w-3.5" />
                </button>
              </TooltipTrigger>
              <TooltipContent side="left">
                <p>{tooltip ?? title}</p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        </div>

        <div className="mt-5 flex items-end justify-between gap-6">
          <div className="min-w-0">
            <div className="text-[2rem] font-bold leading-[0.95] tracking-[-0.04em] text-foreground">
              {formatMetricValue(value)}
            </div>
            <p className="mt-1.5 truncate text-xs font-medium text-muted-foreground">{subtitle}</p>
          </div>

          <svg viewBox="0 0 100 36" className="mb-1 h-11 w-24 shrink-0" aria-hidden="true">
            <path
              d={buildSparklinePath(sparklineData)}
              fill="none"
              stroke={sparklineColor}
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        </div>
      </CardContent>
    </Card>
  )
}
