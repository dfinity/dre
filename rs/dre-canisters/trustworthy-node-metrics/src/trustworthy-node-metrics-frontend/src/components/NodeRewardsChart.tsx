import React, { useEffect, useState } from 'react';
import { ChartData, generateChartData, getLatestRewardRange, LoadingIndicator, NodeMetricsStats, NodePerformanceStats, setNodeRewardsData } from '../utils/utils';
import { Grid } from '@mui/material';
import PerformanceChart from './PerformanceChart';
import { NodeRewards } from '../../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did';
import RewardsInfo, { LinearReductionChart } from './RewardsInfo';
import { Principal } from '@dfinity/principal';

export interface NodeRewardsChartProps {
    node: string;
  }

export const NodeRewardsChart: React.FC<NodeRewardsChartProps> = ({ node }) => {
    const latestRewardRange = getLatestRewardRange();
    const [latestNodeRewards, setLatestNodeRewards] = useState<NodeRewards | null>(null);
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
    const daysAssigned = latestNodeRewards.daily_node_metrics.length;
    const failureRateAvg = Math.round((latestNodeRewards.rewards_computation.failure_rate) * 100)
    const rewardsPercent = Math.round((latestNodeRewards.rewards_computation.rewards_percent) * 100);
    const rewardsReduction = 100 - rewardsPercent;
    const millisecondsPerDay = 24 * 60 * 60 * 1000;
    const daysTotal = Math.round((latestRewardRange.dateEnd.getTime() - latestRewardRange.dateStart.getTime()) / millisecondsPerDay);
    const rewardMultiplier = Math.round((daysAssigned * rewardsPercent + (daysTotal - daysAssigned) * 100) / daysTotal);

    return (
        <>
            <Grid item xs={12} md={6}>
                <NodeMetricsStats stats={latestNodeRewards.rewards_computation} />
            </Grid>
            <Grid item xs={12} md={6}>
                <NodePerformanceStats 
                    failureRateAvg={failureRateAvg.toString().concat("%")} 
                    rewardMultiplier={rewardMultiplier.toString().concat("%")}
                    baseRewardsXDR={latestNodeRewards.node_rate.xdr_permyriad_per_node_per_month.toString()} />
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
