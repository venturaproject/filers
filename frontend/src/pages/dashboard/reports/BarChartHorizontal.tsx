"use client"

import { TrendingUp } from "lucide-react"
import { Bar, BarChart, XAxis, YAxis } from "recharts"

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

export const description = "A horizontal bar chart"

const chartData = [
  { month: "enero", desktop: 186 },
  { month: "febrero", desktop: 305 },
  { month: "marzo", desktop: 237 },
  { month: "abril", desktop: 73 },
  { month: "mayo", desktop: 209 },
  { month: "junio", desktop: 214 },
]

const chartConfig = {
  desktop: {
    label: "Escritorio",
    color: "hsl(var(--chart-1))",
  },
} satisfies ChartConfig

export function BarChartHorizontal() {
  const { t } = useI18n()

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('bar_chart_horizontal_title')}</CardTitle>
        <CardDescription>{t('date_range_jan_june', { year: '2024' })}</CardDescription>
      </CardHeader>
      <CardContent>
        <ChartContainer config={chartConfig}>
          <BarChart
            accessibilityLayer
            data={chartData}
            layout="vertical"
            margin={{
              left: -20,
            }}
          >
            <XAxis type="number" dataKey="desktop" hide />
            <YAxis
              dataKey="month"
              type="category"
              tickLine={false}
              tickMargin={10}
              axisLine={false}
              tickFormatter={(value) => value.slice(0, 3)}
            />
            <ChartTooltip
              cursor={false}
              content={<ChartTooltipContent hideLabel />}
            />
            <Bar dataKey="desktop" fill="var(--color-desktop)" radius={5} />
          </BarChart>
        </ChartContainer>
      </CardContent>
      <CardFooter className="flex-col items-start gap-2 text-sm">
        <div className="flex gap-2 font-medium leading-none">
          {t('trending_up_this_month', { percentage: '5.2%' })} <TrendingUp className="h-4 w-4" />
        </div>
        <div className="leading-none text-muted-foreground">
          {t('chart_visitors_description')}
        </div>
      </CardFooter>
    </Card>
  )
}
