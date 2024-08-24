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

export interface NodeChartProps {
    nodeRewards: NodeRewardsResponse[];
    periodFilter: PeriodFilter;
}

const NodeMetricsStats: React.FC<{ stats: NodeRewardsResponse['rewards_stats'] }> = ({ stats }) => (
    <Box sx={boxStyleWidget('left')}>
        <WidgetNumber value={stats.blocks_proposed.toString()} title="Blocks Proposed Total" />
        <WidgetNumber value={stats.blocks_failed.toString()} title="Blocks Failed Total" />
    </Box>
);

const NodePerformanceStats: React.FC<{ failureRateAvg: string, rewardPercent: string }> = ({ failureRateAvg, rewardPercent }) => (
    <Box sx={boxStyleWidget('right')}>
        <WidgetNumber value={failureRateAvg} title="Failure Rate Assigned" />
        <WidgetNumber value={rewardPercent} title="Rewards Percent Assigned" />
        <WidgetNumber value={"100%"} title="Rewards Percent Unassigned" />
    </Box>
);

export const NodeChart: React.FC<NodeChartProps> = ({ nodeRewards, periodFilter }) => {
    const { node } = useParams();
    
    const nodeMetrics = nodeRewards.find((metrics) => metrics.node_id.toText() === node);
    if (!nodeMetrics) {
        return <p>Node metrics not found</p>;
    }

    const chartDailyData: ChartData[] = generateChartData(periodFilter, nodeMetrics.daily_node_metrics);
    const failureRateAvg = `${Math.round(nodeMetrics.rewards_stats.failure_rate * 100)}%`;
    const rewardPercent = `${Math.round(nodeMetrics.rewards_percent * 100)}%`;

    return (
        <Box sx={{ p: 3 }}>
            <Paper sx={paperStyle}>
                <Grid container spacing={5}>
                    <Grid item xs={12} md={12}>
                        <Typography gutterBottom variant="h5" component="div">
                            {"Node Machine"}
                        </Typography>
                        <Divider></Divider>
                    </Grid>
                    <Grid item xs={12} md={4}>
                        <NodeInfo nodeId={nodeMetrics.node_id.toText()} nodeProviderId={nodeMetrics.node_provider_id.toText()} />
                    </Grid>
                    <Grid item xs={12} md={8}>
                        <WidgetGauge value={nodeMetrics.rewards_percent * 100} title={"Rewards Total"} />
                    </Grid>
                    <Grid item xs={12} md={4}>
                        <NodeMetricsStats stats={nodeMetrics.rewards_stats} />
                    </Grid>
                    <Grid item xs={12} md={8}>
                        <NodePerformanceStats failureRateAvg={failureRateAvg} rewardPercent={rewardPercent} />
                    </Grid>
                    <Grid item xs={12}>
                        <DailyPerformanceChart chartDailyData={chartDailyData} />
                    </Grid>
                </Grid>
            </Paper>
        </Box>
    );
};

export default NodeChart;
