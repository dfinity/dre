import React from 'react';
import { ChartData, formatDateToUTC, generateChartData } from '../utils/utils';
import { WidgetGauge, WidgetNumber } from './Widgets';
import { PeriodFilter } from './FilterBar';
import { Box, Divider, Grid, Paper, Typography } from '@mui/material';
import { useParams } from 'react-router-dom';
import DailyPerformanceChart from './DailyPerformanceChart';
import { paperStyle, boxStyleWidget } from '../Styles';
import { NodeRewardsResponse } from '../../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did';
import RewardsInfo from './RewardsInfo';
import { ExportTable } from './ExportTable';
import InfoFormatter from './NodeInfo';
import { GridColDef, GridRowsProp } from '@mui/x-data-grid';

export interface NodePageProps {
    nodeRewards: NodeRewardsResponse[];
    periodFilter: PeriodFilter;
}

const NodeMetricsStats: React.FC<{ stats: NodeRewardsResponse['rewards_stats'] }> = ({ stats }) => (
    <Box sx={boxStyleWidget('left')}>
        <WidgetNumber value={stats.blocks_proposed.toString()} title="Blocks Proposed Total" />
        <WidgetNumber value={stats.blocks_failed.toString()} title="Blocks Failed Total" />
    </Box>
);

const NodePerformanceStats: React.FC<{ rewardsReduction: string }> = ({ rewardsReduction }) => (
    <Box sx={boxStyleWidget('right')}>
        <WidgetNumber value={rewardsReduction} title="Rewards Reduction Assigned" />
        <WidgetNumber value={"0%"} title="Rewards Reduction Unassigned" />
    </Box>
);

export const NodePage: React.FC<NodePageProps> = ({ nodeRewards, periodFilter }) => {
    const { node } = useParams();
    
    const nodeMetrics = nodeRewards.find((metrics) => metrics.node_id.toText() === node);
    if (!nodeMetrics) {
        return <p>Node metrics not found</p>;
    }

    const chartDailyData: ChartData[] = generateChartData(periodFilter, nodeMetrics.daily_node_metrics);
    const failureRateAvg = Math.round(nodeMetrics.rewards_stats.failure_rate * 100);
    const rewardsPercent = Math.round(nodeMetrics.rewards_percent * 100);
    const rewardsReduction = 100 - rewardsPercent;

    let index = 0;
    const rows: GridRowsProp = nodeMetrics.daily_node_metrics.map((data) => {
        index = index + 1;
        return { 
            id: index,
            col1: new Date(Number(data.ts) / 1000000), 
            col2: data.num_blocks_proposed, 
            col3: data.num_blocks_failed,
            col4: data.failure_rate,
            col5: data.subnet_assigned,
            };
    });
      
    const colDef: GridColDef[] = [
        { field: 'col1', headerName: 'Date (UTC)', width: 200, valueFormatter: (value: Date) => formatDateToUTC(value)},
        { field: 'col2', headerName: 'Blocks Proposed', width: 150 },
        { field: 'col3', headerName: 'Blocks Failed', width: 150 },
        { field: 'col4', headerName: 'Daily Failure Rate', width: 350 , valueFormatter: (value: number) => `${value * 100}%`,},
        { field: 'col5', headerName: 'Subnet Assigned', width: 550 },
        ];

    return (
        <Box sx={{ p: 3 }}>
            <Paper sx={paperStyle}>
                <Grid container spacing={3}>
                    <Grid item xs={12} md={12}>
                        <Typography gutterBottom variant="h5" component="div">
                            {"Node Machine"}
                        </Typography>
                        <Divider/>
                    </Grid>
                    <Grid item xs={12} md={4}>
                        <InfoFormatter name={"Node ID"} value={nodeMetrics.node_id.toText()} />
                        <InfoFormatter name={"Node Provider ID"} value={nodeMetrics.node_provider_id.toText()} />
                    </Grid>
                    <Grid item xs={12} md={8}>
                        <WidgetGauge value={rewardsPercent} title={"Rewards Total"} />
                    </Grid>
                    <Grid item xs={12} md={4}>
                        <NodeMetricsStats stats={nodeMetrics.rewards_stats} />
                    </Grid>
                    <Grid item xs={12} md={8}>
                        <NodePerformanceStats rewardsReduction={rewardsReduction.toString().concat("%")} />
                    </Grid>
                    <Grid item xs={12}>
                        <DailyPerformanceChart chartDailyData={chartDailyData} />
                    </Grid>
                    <Grid item xs={12}>
                        <RewardsInfo failureRate={failureRateAvg} rewardReduction={rewardsReduction}/>
                    </Grid>
                    <Grid item xs={12} md={12}>
                        <ExportTable colDef={colDef} rows={rows}/>
                    </Grid>
                </Grid>
            </Paper>
        </Box>
    );
};

export default NodePage;
