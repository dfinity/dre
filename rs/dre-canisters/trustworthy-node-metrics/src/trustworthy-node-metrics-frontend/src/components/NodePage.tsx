import React, { useEffect, useState } from 'react';
import { ChartData, formatDateToUTC, generateChartData, LoadingIndicator, setNodeRewardsData } from '../utils/utils';
import { WidgetNumber } from './Widgets';
import { PeriodFilter } from './FilterBar';
import { Box, Divider, Grid, Paper, Typography } from '@mui/material';
import { Link, useParams } from 'react-router-dom';
import RewardChart from './RewardChart';
import { paperStyle, boxStyleWidget } from '../Styles';
import { NodeRewardsResponse } from '../../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did';
import RewardsInfo, { LinearReductionChart } from './RewardsInfo';
import { ExportTable } from './ExportTable';
import InfoFormatter from './NodeInfo';
import { GridColDef, GridRowsProp } from '@mui/x-data-grid';
import { Principal } from '@dfinity/principal';

export interface NodePageProps {
    periodFilter: PeriodFilter;
}

const NodeMetricsStats: React.FC<{ stats: NodeRewardsResponse['rewards_computation'] }> = ({ stats }) => (
    <Box sx={boxStyleWidget('left')}>
        <WidgetNumber value={stats.blocks_proposed.toString()} title="Blocks Proposed Total" />
        <WidgetNumber value={stats.blocks_failed.toString()} title="Blocks Failed Total" />
    </Box>
);

const NodePerformanceStats: React.FC<{ rewardsReduction: string, rewardsPercent: string }> = ({ rewardsReduction, rewardsPercent }) => (
    <Box sx={boxStyleWidget('right')}>
        <WidgetNumber value={rewardsReduction} title="Rewards Reduction Assigned" />
        <WidgetNumber value={"0%"} title="Rewards Reduction Unassigned" />
        <WidgetNumber value={rewardsPercent} title="Rewards Total" sxValue={{ color: '#FFCC00' }} />
    </Box>
);

export const NodePage: React.FC<NodePageProps> = ({ periodFilter }) => {
    const { node } = useParams();

    const [nodeRewards, setNodeRewards] = useState<NodeRewardsResponse[]>([]);
    const [isLoading, setIsLoading] = useState(true);

    useEffect(() => {
        if (node) {
            setNodeRewardsData(periodFilter, [Principal.fromText(node)], [], setNodeRewards, setIsLoading);
        }    
    }, [periodFilter]);
    
    if (isLoading) {
        return <LoadingIndicator />;
    }

    if (nodeRewards.length == 0) {
        return <p>No metrics for the time period selected</p>;
    }

    const rewards = nodeRewards[0];
    const chartDailyData: ChartData[] = generateChartData(periodFilter, rewards.daily_node_metrics);
    const failureRateAvg = Math.round(rewards.rewards_computation.failure_rate * 100);
    const rewardsPercent = Math.round(rewards.rewards_computation.rewards_percent * 100);
    const rewardsReduction = 100 - rewardsPercent;

    const rows: GridRowsProp = rewards.daily_node_metrics.map((data, index) => {
        return { 
            id: index + 1,
            col1: new Date(Number(data.ts) / 1000000), 
            col2: Number(data.num_blocks_proposed), 
            col3: Number(data.num_blocks_failed),
            col4: data.failure_rate,
            col5: data.subnet_assigned.toText(),
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
                        <InfoFormatter name={"Node ID"} value={rewards.node_id.toText()} />
                        <Typography gutterBottom variant="subtitle1" component="div">
                            Node Provider ID
                        </Typography>
                        <Typography gutterBottom variant="subtitle2" sx={{ color: 'text.disabled' }} component="div">
                        <Link to={`/providers/${rewards.node_provider_id.toText()}`} className="custom-link">
                            {rewards.node_provider_id.toText()}
                        </Link>
                        </Typography>
                    </Grid>
                    <Grid item xs={12} md={12}>
                    <Typography variant="h6" component="div" >
                        Reward Metrics
                    </Typography>
                    <Divider/>
                    </Grid>
                    <Grid item xs={12} md={4}>
                        <NodeMetricsStats stats={rewards.rewards_computation} />
                    </Grid>
                    <Grid item xs={12} md={8}>
                        <NodePerformanceStats rewardsReduction={rewardsReduction.toString().concat("%")} rewardsPercent={rewardsPercent.toString().concat("%")} />
                    </Grid>
                    <Grid item xs={12}>
                        <RewardChart chartDailyData={chartDailyData} />
                    </Grid>
                    <Grid item xs={12} md={6}>
                        <LinearReductionChart failureRate={failureRateAvg} rewardReduction={rewardsReduction} />
                    </Grid>
                    <Grid item xs={12}>
                        <RewardsInfo/>
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
