import React from 'react';
import { ChartData, generateChartData } from '../utils/utils';
import { WidgetGauge, WidgetNumber } from './Widgets';
import { PeriodFilter } from './FilterBar';
import { Box, Divider, Grid, Paper, Typography } from '@mui/material';
import { useParams } from 'react-router-dom';
import DailyPerformanceChart from './DailyPerformanceChart';
import NodeInfo from './NodeInfo';
import { paperStyle, boxStyleWidget } from '../Styles';
import { NodeRewardsResponse } from '../../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did';
import RewardsInfo, { LinearReductionChart } from './RewardsInfo';
import ExportCustomToolbar from './NodeDailyData';
import { ExportCustomToolbar } from './NodeDailyData';

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
                        <NodeInfo nodeId={nodeMetrics.node_id.toText()} nodeProviderId={nodeMetrics.node_provider_id.toText()} />
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
                        <ExportCustomToolbar chartDailyData={chartDailyData}/>
                    </Grid>
                    <Grid item xs={12} md={12}>
                        <ExportCustomToolbar />
                    </Grid>
                </Grid>
            </Paper>
        </Box>
    );
};

export default NodePage;
