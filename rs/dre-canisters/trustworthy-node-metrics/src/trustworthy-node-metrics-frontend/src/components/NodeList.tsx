import React, { useState } from 'react';
import { Box, Grid, Paper, Stack, Typography, Autocomplete, TextField } from '@mui/material';
import { axisClasses, BarChart } from '@mui/x-charts';
import { ChartData, DailyData, DashboardNodeMetrics } from '../models/NodeMetrics';
import { styled } from '@mui/material/styles';
import Divider from '@mui/material/Divider';
import { PeriodFilter } from './FilterBar';
import { generateChartData, getFormattedDates, transformDailyData } from '../utils/utils';
import FailureRateArc, { RewardsArc } from './Gauge';

export const Root = styled('div')(({ theme }) => ({
    width: '100%',
    ...theme.typography.body2,
    color: theme.palette.text.secondary,
    '& > :not(style) ~ :not(style)': {
      marginTop: theme.spacing(2),
    },
}));

export interface NodeListProps {
    dashboardNodeMetrics: DashboardNodeMetrics[],
    periodFilter: PeriodFilter
}

function renderChart(nodeId: string, dailyData: DailyData[], failureRateAvg: number, periodFilter: PeriodFilter): React.ReactNode {

    const chartDailyData: ChartData[] = generateChartData(periodFilter, dailyData);

    return ( 
    <Paper sx={{ p: 2, backgroundColor: '#11171E', borderRadius: '10px', color: 'white' }}>
        <Box sx={{ display: 'flex', alignItems: 'center' }}>
            <Typography gutterBottom variant="h6" component="div">
                {nodeId}
            </Typography>
            <Box sx={{ ml: 'auto', display: 'flex', alignItems: 'center' }}>
                {FailureRateArc(Math.round(failureRateAvg))}
                {RewardsArc(Math.round(failureRateAvg))}
            </Box>
        </Box>
    <Box sx={{ p: 2 }}>
        <Divider />
        <BarChart
            borderRadius={9}
            sx={{
                p: 2,
                [`.${axisClasses.left} .${axisClasses.label}`]: {
                    transform: 'translateX(-25px)',
                },
            }}
            slotProps={{ legend: { hidden: true } }}
            xAxis={[{ 
                scaleType: 'band',
                dataKey: 'date',
                valueFormatter: (value: Date) => value.toLocaleDateString('UTC', { month: 'short', day: 'numeric' }).replace(" ", "\n")
            }]}
            yAxis={[
                {
                    valueFormatter: value => `${value}%`,
                    min: 0,
                    max: 100,
                },
            ]}
            series={[
                { dataKey: 'failureRate', label: "Failure Rate", color: '#FF6347', 
                    valueFormatter: (value: number | null) => value ? `${value}%` : 'Not assigned' },
            ]}
            dataset={chartDailyData.map(entry => ({
                date: entry.date,
                failureRate: entry.failureRate,
            }))}

            height={400}
        />
    </Box>
</Paper>
    )
}

export const NodeList: React.FC<NodeListProps> = ({ dashboardNodeMetrics, periodFilter }) => {
    const [prevItems, setPrevItems] = useState(dashboardNodeMetrics);
    const [filteredMetrics, setFilteredMetrics] = useState(prevItems);

    if (dashboardNodeMetrics !== prevItems) {
        setPrevItems(dashboardNodeMetrics);
        setFilteredMetrics(dashboardNodeMetrics)
      }

    const handleSearchChange = (event: unknown, value: string | null) => {
        if (value) {
            const filtered = dashboardNodeMetrics.filter(node => node.nodeId.includes(value));
            setFilteredMetrics(filtered);
        } else {
            setFilteredMetrics(dashboardNodeMetrics);
        }
    };

    return (
        <React.Fragment>
            <Root>
                <Box sx={{ p: 2 }}>
                    <Autocomplete
                        freeSolo
                        id="node-search"
                        options={dashboardNodeMetrics.map((node) => node.nodeId)}
                        onInputChange={handleSearchChange}
                        renderInput={(params) => (
                            <TextField {...params} label="Search Node" variant="outlined" />
                        )}
                    />
                </Box>
                <Grid container spacing={2}>
                {filteredMetrics.slice(0, 10).map(({ nodeId, dailyData, failureRateAvg }, index) => (
                    <Grid item xs={6} key={index}>
                        {renderChart(nodeId, dailyData, failureRateAvg, periodFilter)}
                    </Grid>
                ))}
                </Grid>
            </Root>
        </React.Fragment>
    );
};
