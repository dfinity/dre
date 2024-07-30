import React, { useState } from 'react';
import { Box, Grid, Paper, Stack, Typography, Autocomplete, TextField } from '@mui/material';
import { axisClasses, BarChart } from '@mui/x-charts';
import { DashboardNodeMetrics } from '../models/NodeMetrics';
import { styled } from '@mui/material/styles';
import Divider from '@mui/material/Divider';
import { PeriodFilter } from './FilterBar';
import { getFormattedDates, transformDailyData } from '../utils/utils';
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

export const NodeList: React.FC<NodeListProps> = ({ dashboardNodeMetrics, periodFilter }) => {
    const [filteredMetrics, setFilteredMetrics] = useState(dashboardNodeMetrics);

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
                {filteredMetrics.slice(0, 10).map(({ nodeId, dailyData, failureRateAvg }, index) => (
                    <Paper key={index} sx={{ p: 2, backgroundColor: '#11171E', borderRadius: '10px', color: 'white' }}>
                        <Box sx={{ p: 2 }}>
                            <Typography gutterBottom variant="h6" component="div">
                                {nodeId}
                            </Typography>
                        </Box>
                        <Box sx={{ p: 2 }}>
                            <Divider />
                            <Grid container spacing={2}>
                                <Grid item xs={10}>
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
                                            data: getFormattedDates(periodFilter),
                                        }]}
                                        yAxis={[
                                            {
                                                valueFormatter: value => `${value}%`,
                                                label: 'Failure Rate',
                                                min: 0,
                                                max: 100,
                                            },
                                        ]}
                                        series={[
                                            { dataKey: 'failureRate', label: "Failure Rate (%)", color: '#FF6347' },
                                        ]}
                                        dataset={transformDailyData(dailyData)}
                                        height={400}
                                    />
                                </Grid>
                                <Grid item xs={2}>
                                    {FailureRateArc(Math.round(failureRateAvg))}
                                    {RewardsArc(Math.round(failureRateAvg))}
                                </Grid>
                            </Grid>
                        </Box>
                    </Paper>
                ))}
            </Root>
        </React.Fragment>
    );
};
