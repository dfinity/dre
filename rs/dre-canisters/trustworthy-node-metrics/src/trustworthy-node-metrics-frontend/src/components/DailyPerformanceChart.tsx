import React from 'react';
import Typography from '@mui/material/Typography';
import { 
  BarPlot, 
  LinePlot, 
  MarkPlot, 
  ChartsAxisHighlight, 
  ChartsTooltip, 
  ChartsLegend, 
  ChartsXAxis, 
  ChartsYAxis,
  ResponsiveChartContainer
} from '@mui/x-charts'; // Update paths
import { ChartData } from '../utils/utils';

interface DailyPerformanceChartProps {
  chartDailyData: ChartData[];
}

const DailyPerformanceChart: React.FC<DailyPerformanceChartProps> = ({ chartDailyData }) => {
  const maxBlocks = Math.max(
    ...chartDailyData.map(entry => entry.dailyNodeMetrics ? 
            Number(entry.dailyNodeMetrics.num_blocks_proposed + entry.dailyNodeMetrics.num_blocks_failed) : 0)
    );
  const xAxisConfig = [
    {
      id: 'x-axis-id',
      scaleType: 'band' as const,
      dataKey: 'date',
      categoryGapRatio: 0.4,
      barGapRatio: 0.3,
      valueFormatter: (value: Date) => value
        .toLocaleDateString('UTC', { month: 'short', day: 'numeric' })
        .replace(" ", "\n"),
    }
  ];

  const yAxisConfig = [
    {
      id: 'y-axis-id',
      valueFormatter: (value: number) => `${value}`,
    }
  ];

  const seriesConfig = [
    {
      dataKey: 'blocksProposed',
      label: 'Proposed Blocks',
      color: '#4CAF79',
      stack: 'total',
      type: 'bar' as const,
      valueFormatter: (value: number | null) => value ? `${value}` : '0',
    },
    {
      dataKey: 'blocksFailed',
      label: 'Failed Blocks',
      color: '#FF6347',
      stack: 'total',
      type: 'bar' as const,
      valueFormatter: (value: number | null) => value ? `${value}` : '0',
    },
    {
      dataKey: 'unassigned',
      label: 'Unassigned',
      color: 'white',
      area: true,
      stack: 'total',
      type: 'line' as const,
      valueFormatter: (value: number | null) => (value ? 'Yes' : ''),
    }
  ];

  const dataset = chartDailyData.map((entry) => ({
    date: entry.date,
    blocksProposed: entry.dailyNodeMetrics ? Number(entry.dailyNodeMetrics.num_blocks_proposed) : null,
    blocksFailed: entry.dailyNodeMetrics ? Number(entry.dailyNodeMetrics.num_blocks_failed) : null,
    unassigned: entry.dailyNodeMetrics ? null : maxBlocks / 2,
  }));

  const legendConfig = {
    legend: {
      labelStyle: {
        fontSize: 14,
        fill: 'white',
      },
      direction: 'row' as const,
      position: { vertical: 'top', horizontal: 'right' } as const,
      itemMarkWidth: 18,
      itemMarkHeight: 18,
      markGap: 6,
      itemGap: 10,
    },
  };

  return (
    <div>
      <Typography variant="h6" component="div">
        Daily Performance
      </Typography>
      <ResponsiveChartContainer
        xAxis={xAxisConfig}
        yAxis={yAxisConfig}
        series={seriesConfig}
        dataset={dataset}
        height={400}
      >
        <BarPlot borderRadius={9} />
        <LinePlot />
        <MarkPlot />
        <ChartsAxisHighlight />
        <ChartsTooltip />
        <ChartsXAxis position="bottom" axisId="x-axis-id" />
        <ChartsYAxis position="left" axisId="y-axis-id" />
      </ResponsiveChartContainer>
    </div>
  );
};

export default DailyPerformanceChart;
