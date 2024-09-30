import React, { useEffect, useState } from 'react';
import { ChartData, generateChartData, getLatestRewardRange, LoadingIndicator, NodeMetricsStats, NodePerformanceStats, setNodeRewardsData } from '../utils/utils';
import { Grid, Typography } from '@mui/material';
import PerformanceChart from './PerformanceChart';
import { NodeRewards } from '../../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did';
import RewardsInfo, { LinearReductionChart } from './RewardsInfo';
import { Principal } from '@dfinity/principal';
import { ExportTable } from './ExportTable';
import { GridColDef, GridRowsProp } from '@mui/x-data-grid';

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
    const failureRateAvg = Math.round((latestNodeRewards.rewards_computation.failure_rate) * 100)
    const rewardsMultiplier = Math.round((latestNodeRewards.rewards_computation.rewards_multiplier) * 100);
    const rewardsReduction = 100 - rewardsMultiplier;
    const rows: GridRowsProp = latestNodeRewards.rewards_computation.computation_log.map((data, index) => {
        return { 
            id: index,
            col0: index,
            col1: data.reason, 
            col2: data.operation,
            col3: data.result
            };
    });
    const colDef: GridColDef[] = [
        { field: 'col0', headerName: 'Step', width: 100},
        { field: 'col1', headerName: 'Description', width: 300},
        { field: 'col2', headerName: 'Operation', width: 500 },
        { field: 'col3', headerName: 'Result', width: 200 },
        ];

    return (
        <>
            <Grid item xs={12} md={6}>
                <NodeMetricsStats stats={latestNodeRewards.rewards_computation} />
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
            <Grid item xs={12} md={12}>
                <Typography variant="body1" gutterBottom>
                    Computation Log
                </Typography>
                <ExportTable colDef={colDef} rows={rows}/>
            </Grid>
        </>
    );
};

export default NodeRewardsChart;
