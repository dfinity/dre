import React, { useState } from 'react';
import { Box, Grid, Paper, Typography, Autocomplete, TextField } from '@mui/material';
import { axisClasses, BarChart } from '@mui/x-charts';
import { ChartData, DashboardNodeRewards } from '../models/NodeMetrics';
import { styled } from '@mui/material/styles';
import Divider from '@mui/material/Divider';
import { PeriodFilter } from './FilterBar';
import { generateChartData} from '../utils/utils';
import FailureRateArc, { RewardsArc } from './Gauge';
import { Principal } from '@dfinity/principal';
import { DailyNodeMetrics } from '../../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did';

export const Root = styled('div')(({ theme }) => ({
    width: '100%',
    ...theme.typography.body2,
    color: theme.palette.text.secondary,
    '& > :not(style) ~ :not(style)': {
      marginTop: theme.spacing(2),
    },
}));

export interface NodeListProps {
    dashboardNodeMetrics: DashboardNodeRewards[],
    periodFilter: PeriodFilter
}

function renderChart(
    nodeId: Principal, 
    dailyData: DailyNodeMetrics[], 
    failureRateAvg: number, 
    rewardsNoPenalty: number, 
    rewardsWithPenalty: number, 
    periodFilter: PeriodFilter): React.ReactNode {

    const chartDailyData: ChartData[] = generateChartData(periodFilter, dailyData);

    return ( 
    <Paper sx={{ p: 2, backgroundColor: '#11171E', borderRadius: '10px', color: 'white' }}>
        <Box sx={{ display: 'flex', alignItems: 'center' }}>
            <Typography gutterBottom variant="h6" component="div">
                {nodeId.toText()}
            </Typography>
            <Box sx={{ ml: 'auto', display: 'flex', alignItems: 'center' }}>
                {FailureRateArc(failureRateAvg)}
                {RewardsArc(rewardsNoPenalty, 'Rewards No Penalty')}
                {RewardsArc(rewardsWithPenalty, 'Rewards With Penalty')}
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
            const filtered = dashboardNodeMetrics.filter(node => node.nodeId.toText().includes(value));
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
                        options={dashboardNodeMetrics.map((node) => node.nodeId.toText())}
                        onInputChange={handleSearchChange}
                        renderInput={(params) => (
                            <TextField {...params} label="Search Node" variant="outlined" />
                        )}
                    />
                </Box>
                <Grid container spacing={2}>
                {filteredMetrics.slice(0, 20).map(
                    ({ nodeId, dailyData, failureRateAvg, rewardsNoPenalty, rewardsWithPenalty }, index) => (
                    <Grid item xs={6} key={index}>
                        {renderChart(nodeId, dailyData, failureRateAvg, rewardsNoPenalty, rewardsWithPenalty, periodFilter)}
                    </Grid>
                ))}
                </Grid>
            </Root>
        </React.Fragment>
    );
};
