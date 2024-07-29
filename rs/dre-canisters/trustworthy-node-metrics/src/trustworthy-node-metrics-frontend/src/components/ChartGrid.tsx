import React, { useState } from 'react';
import { Box, CircularProgress, Grid, Paper, Stack, Typography } from '@mui/material';
import { axisClasses, BarChart, StackOrderType } from '@mui/x-charts';
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
    const currentDate = item.date;
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


interface ChartGridProps {
    data: NodeMetrics[]
  }

  export const ChartGrid: React.FC<ChartGridProps> = ({ data }) => {
    const [processedData, setProcessedData] = useState<any[]>([]);
    const [isLoading, setIsLoading] = useState(true);

    React.useEffect(() => {
        const processData = () => {
            const groupedItems = groupBy(data, 'node_id');

            const processed = Object.keys(groupedItems).map(nodeId => {
                const items = groupedItems[nodeId];
                const dailyData = calculateDailyValues(items);

                return {
                    nodeId,
                    title: nodeId.split('-')[0],
                    subnetId: items[0].subnet_id,
                    dailyData: dailyData,
                    failureRateSum: dailyData.reduce((sum, item) => sum + item.failureRate, 0),
                    failureRateAvg: computeAverageFailureRate(dailyData.map(elem => elem.failureRate)),
                };
            }).sort((a, b) => b.failureRateSum - a.failureRateSum);

            setProcessedData(processed);
            setIsLoading(false);
        };

        processData();
    }, [data]);

    return (
        <Stack spacing={10}>
            {isLoading ? (
                <Box sx={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100%' }}>
                    <CircularProgress color="inherit" />
                </Box>
            ) : (
                processedData.slice(0, 10).map(({ nodeId, title, subnetId, dailyData, failureRateAvg }, index) => (
                    <React.Fragment key={index}>
                        <Root>
                        <Paper sx={{ p: 2, backgroundColor: '#11171E', borderRadius: '10px', color: 'white' }}>
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
                                        xAxis={[{ scaleType: 'band', dataKey: 'date', valueFormatter: date => new Date(date).toLocaleDateString('en-US', { month: 'short', day: 'numeric' }).replace(" ", "\n") }]}
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
                ))
            )}
        </Stack>
    );
};
interface StackedChartProps {
    data: NodeMetrics[];
    name: string | null;
  }

  export const StackedChart: React.FC<StackedChartProps> = ({ data, name }) => {
    const [stackOrder, setStackOrder] = useState<StackOrderType>('ascending');
    const [isLoading, setIsLoading] = useState(true);
    const [modifiedSeries, setModifiedSeries] = useState<any[]>([]);

    React.useEffect(() => {
        const processData = () => {
            const nodesGrouped = groupBy(data, 'node_id');

            const chartData = Object.keys(nodesGrouped).flatMap(nodeId => {
                const items = nodesGrouped[nodeId];
                const dailyData = calculateDailyValues(items);
                return {
                    data: dailyData.map(daily => daily.failureRate),
                    stack: 'A',
                    label: nodeId,
                };
            });

            const series = [{ ...chartData[0], stackOrder }, ...chartData.slice(1)];

            setModifiedSeries(series);
            setIsLoading(false);
        };

        processData();
    }, [data, stackOrder]);

    return (
        <Root>
            <Paper sx={{ p: 2, backgroundColor: '#11171E', borderRadius: '10px', color: 'white' }}>
                <Box sx={{ p: 2 }}>
                    <Stack direction="row" justifyContent="space-between" alignItems="center">
                        <Typography gutterBottom variant="h6" component="div">
                            Subnet: {name}
                        </Typography>
                    </Stack>
                </Box>
                <Divider />
                <Box sx={{ p: 2 }}>
                    {isLoading ? (
                        <Box sx={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100%' }}>
                            <CircularProgress color="inherit" />
                        </Box>
                    ) : (
                        <Grid container spacing={1}>
                            <Grid item xs={12}>
                                <BarChart
                                    slotProps={{ legend: { hidden: true } }}
                                    yAxis={[
                                        {
                                            valueFormatter: value => `${value}%`,
                                            label: 'Sum nodes failure rate',
                                        },
                                    ]}
                                    sx={{
                                        p: 2,
                                        [`.${axisClasses.left} .${axisClasses.label}`]: {
                                            transform: 'translateX(-35px)',
                                        },
                                    }}
                                    borderRadius={9}
                                    series={modifiedSeries}
                                    height={800}
                                />
                            </Grid>
                        </Grid>
                    )}
                </Box>
            </Paper>
        </Root>
    );
};

  export const TestChart: React.FC<StackedChartProps> = ({ data, name }) => {
  
    return (
        <Root>
        <Paper sx={{ p: 2, backgroundColor: '#11171E', borderRadius: '10px', color: 'white'}}>
        <Box sx={{ p: 2 }}>
            <Stack direction="row" justifyContent="space-between" alignItems="center">
            <Typography gutterBottom variant="h6" component="div">
                Subnet: {name}
            </Typography>
            </Stack>
        </Box>
        <Divider />
        </Paper>               
        </Root>

    );
  };
