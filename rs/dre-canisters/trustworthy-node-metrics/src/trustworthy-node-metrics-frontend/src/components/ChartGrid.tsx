import React from 'react';
import { Box, Grid, Paper, Stack, Typography } from '@mui/material';
import { BarChart } from '@mui/x-charts';
import { NodeMetrics } from '../models/NodeMetrics';
import { styled } from '@mui/material/styles';
import Divider from '@mui/material/Divider';

const Root = styled('div')(({ theme }) => ({
    width: '100%',
    ...theme.typography.body2,
    color: theme.palette.text.secondary,
    '& > :not(style) ~ :not(style)': {
      marginTop: theme.spacing(2),
    },
  }));

  
interface ChartGridProps {
  data: NodeMetrics[];
}

function groupBy<T, K extends keyof T>(items: T[], key: K): Record<string, T[]> {
  return items.reduce((result, item) => {
    const groupKey = String(item[key]);
    if (!result[groupKey]) {
      result[groupKey] = [];
    }
    result[groupKey].push(item);
    return result;
  }, {} as Record<string, T[]>);
}

const calculateDailyValues = (items: NodeMetrics[]) => {
  const dailyValues = [];
  let previousTotals = { num_block_failures_total: 0.0, num_blocks_proposed_total: 0.0 };
  
  items.sort((a, b) => a.date.getTime() - b.date.getTime());

  for (const item of items) {
    const currentDate = item.date.toDateString();
    const currentTotals = {
      num_block_failures_total: Number(item.num_block_failures_total),
      num_blocks_proposed_total: Number(item.num_blocks_proposed_total),
    };

    if (previousTotals.num_block_failures_total || previousTotals.num_blocks_proposed_total) {
        const dailyFailures = currentTotals.num_block_failures_total - previousTotals.num_block_failures_total;
        const dailyProposed = currentTotals.num_blocks_proposed_total - previousTotals.num_blocks_proposed_total;

        dailyValues.push({
            date: currentDate,
            num_block_failures_day: currentTotals.num_block_failures_total - previousTotals.num_block_failures_total,
            num_blocks_proposed_day: currentTotals.num_blocks_proposed_total - previousTotals.num_blocks_proposed_total,
            failureRate: dailyFailures / (dailyProposed + dailyFailures) * 100,
        });
    }

    previousTotals = currentTotals;
  }

  return dailyValues;
};


const computeAverageFailureRate = (data: number[]): number => {
    if (data.length === 0) return 0;
    const sum = data.reduce((acc, val) => acc + val, 0);
    return sum / data.length;
  };

const ChartGrid: React.FC<ChartGridProps> = ({ data }) => {
  const groupedItems = groupBy(data, 'node_id');

  const chartData = Object.keys(groupedItems).map(nodeId => {
    const items = groupedItems[nodeId];
    const dailyData = calculateDailyValues(items);

    return {
      nodeId,
      title: nodeId.split('-')[0],
      subnetId: items[0].subnet_id,
      dailyData: dailyData,
      failureRateSum: dailyData.reduce((sum, item) => sum + item.failureRate, 0),
      failureRateAvg: computeAverageFailureRate(dailyData.map((elem) => elem.failureRate)),
    };
  }).sort((a, b) => b.failureRateSum - a.failureRateSum);

  return (
    <Stack spacing={10}>
      {chartData.map(({ nodeId, title, subnetId, dailyData, failureRateAvg }, index) => (
        <React.Fragment key={index}>
            <Root>
            <Paper sx={{ p: 2, backgroundColor: '#11171E', borderRadius: '10px', color: 'white'}}>
            <Box sx={{ p: 2 }}>
                <Stack direction="row" justifyContent="space-between" alignItems="center">
                <Typography gutterBottom variant="h6" component="div">
                    {nodeId}
                </Typography>
                </Stack>
            </Box>
            <Divider />
            <Box sx={{ p: 2 }}>
            <Grid container spacing={1}>
                <Grid item xs={12}>
                    <BarChart
                        borderRadius={9}
                        xAxis={[{ scaleType: 'band', dataKey: 'date', 
                            valueFormatter: (date) => {
                                const formatted = new Date(date).toLocaleDateString('en-US', { month: 'short', day: 'numeric' }).replace(" ", "\n")
                                return formatted
                        }}]}
                        series={[
                            { dataKey: 'failureRate', label: "Failure Rate (%)", color: '#FF6347' },
                        ]}
                        dataset={dailyData}
                        height={400}
                        />
                </Grid>
            </Grid>
            </Box>
            </Paper>               
            </Root>
        </React.Fragment>
      ))}
    </Stack>
    
  );
};

export default ChartGrid;
