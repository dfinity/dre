import React from 'react';
import { 
  BarPlot, 
  LinePlot, 
  MarkPlot, 
  ChartsAxisHighlight, 
  ChartsTooltip, 
  ChartsXAxis, 
  ChartsYAxis,
  ResponsiveChartContainer
} from '@mui/x-charts'; // Update paths
import { ChartData } from '../utils/utils';

interface RewardChartProps {
  chartDailyData: ChartData[];
}

const RewardChart: React.FC<RewardChartProps> = ({ chartDailyData }) => {
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

  return (
    <div>
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

export default RewardChart;
