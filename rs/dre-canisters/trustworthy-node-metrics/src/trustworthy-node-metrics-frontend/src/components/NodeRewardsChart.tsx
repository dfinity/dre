import React, { useEffect, useState } from 'react';
import { ChartData, generateChartData, getLatestRewardRange, LoadingIndicator, NodeMetricsStats, NodePerformanceStats, setNodeRewardsData } from '../utils/utils';
import { Grid, Typography } from '@mui/material';
import PerformanceChart from './PerformanceChart';
import { NodeRewardsMultiplier } from '../../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did';
import RewardsInfo, { LinearReductionChart } from './RewardsInfo';
import { Principal } from '@dfinity/principal';

export interface NodeRewardsChartProps {
    node: string;
  }

export const NodeRewardsChart: React.FC<NodeRewardsChartProps> = ({ node }) => {
    const latestRewardRange = getLatestRewardRange();
    const [latestNodeRewards, setLatestNodeRewards] = useState<NodeRewardsMultiplier | null>(null);
    const [isLoading, setIsLoading] = useState(true);

    useEffect(() => {
        if (node) {
            setNodeRewardsData(latestRewardRange, Principal.fromText(node), setLatestNodeRewards, setIsLoading);
        }    
    }, [node]);


    if (isLoading) {
        return <LoadingIndicator />;
    }

    if (!latestNodeRewards) {
        return <p>No latestNodeRewards</p>;
    }

    const rewardsDailyData: ChartData[] = generateChartData(latestRewardRange, latestNodeRewards.daily_node_metrics);
    const failureRateAvg = Math.round((latestNodeRewards.rewards_multiplier_stats.failure_rate) * 100)
    const rewardsMultiplier = Math.round((latestNodeRewards.rewards_multiplier) * 100);
    const rewardsReduction = 100 - rewardsMultiplier;

    return (
        <>
            <Grid item xs={12} md={6}>
                <NodeMetricsStats stats={latestNodeRewards.rewards_multiplier_stats} />
            </Grid>
            <Grid item xs={12} md={6}>
                <NodePerformanceStats 
                    rewardMultiplier={rewardsMultiplier.toString().concat("%")}
                    baseRewardsXDR={(Number(latestNodeRewards.node_rate.xdr_permyriad_per_node_per_month) / 10000).toString()} />
            </Grid>
            <Grid item xs={12} md={6}>
                <PerformanceChart chartDailyData={rewardsDailyData} />
            </Grid>
            <Grid item xs={12} md={6}>
                <LinearReductionChart failureRate={failureRateAvg} rewardReduction={rewardsReduction} />
            </Grid>
            <Grid item xs={12} md={12}>
                <RewardsInfo/>
            </Grid>
        </>
    );
};

export default NodeRewardsChart;
