"use client"

import { TrendingUp } from "lucide-react"
import { CartesianGrid, Line, LineChart, XAxis } from "recharts"

import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import {
  ChartConfig,
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
} from "@/components/ui/chart"
import { useI18n } from "@/i18n/context"

export const description = "A multiple line chart"

const chartData = [
  { month: "enero", desktop: 186, mobile: 80 },
  { month: "febrero", desktop: 305, mobile: 200 },
  { month: "marzo", desktop: 237, mobile: 120 },
  { month: "abril", desktop: 73, mobile: 190 },
  { month: "mayo", desktop: 209, mobile: 130 },
  { month: "junio", desktop: 214, mobile: 140 },
]

const chartConfig = {
  desktop: {
    label: "Escritorio",
    color: "hsl(var(--chart-1))",
  },
  mobile: {
    label: "Móvil",
    color: "hsl(var(--chart-2))",
  },
} satisfies ChartConfig

export function LineChartMultiple() {
  const { t } = useI18n()

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('line_chart_multiple_title')}</CardTitle>
        <CardDescription>{t('date_range_jan_june', { year: '2024' })}</CardDescription>
      </CardHeader>
      <CardContent>
        <ChartContainer config={chartConfig}>
          <LineChart
            accessibilityLayer
            data={chartData}
            margin={{
              left: 12,
              right: 12,
            }}
          >
            <CartesianGrid vertical={false} />
            <XAxis
              dataKey="month"
              tickLine={false}
              axisLine={false}
              tickMargin={8}
              tickFormatter={(value) => value.slice(0, 3)}
            />
            <ChartTooltip cursor={false} content={<ChartTooltipContent />} />
            <Line
              dataKey="desktop"
              type="monotone"
              stroke="var(--color-desktop)"
              strokeWidth={2}
              dot={false}
            />
            <Line
              dataKey="mobile"
              type="monotone"
              stroke="var(--color-mobile)"
              strokeWidth={2}
              dot={false}
            />
          </LineChart>
        </ChartContainer>
      </CardContent>
      <CardFooter>
        <div className="flex w-full items-start gap-2 text-sm">
          <div className="grid gap-2">
            <div className="flex items-center gap-2 font-medium leading-none">
              {t('trending_up_this_month', { percentage: '5.2%' })} <TrendingUp className="h-4 w-4" />
            </div>
            <div className="flex items-center gap-2 leading-none text-muted-foreground">
              {t('chart_visitors_description')}
            </div>
          </div>
        </div>
      </CardFooter>
    </Card>
  )
}
