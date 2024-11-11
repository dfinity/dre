import React, { useEffect, useState } from 'react';
import { getLatestRewardRange, LoadingIndicator, setNodeProviderRewardsData } from '../utils/utils';
import { Box, Grid, Typography } from '@mui/material';
import { Principal } from '@dfinity/principal';
import { NodeProviderRewards } from '../../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did';
import { WidgetNumber } from './Widgets';
import { boxStyleWidget } from '../Styles';
import { ExportTable } from './ExportTable';
import { GridColDef, GridRowsProp } from '@mui/x-data-grid';

export interface NodeProviderRewardsChartProps {
    provider: string;
  }

export const NodeProviderRewardsChart: React.FC<NodeProviderRewardsChartProps> = ({ provider }) => {
    const latestRewardRange = getLatestRewardRange();
    const [latestProviderRewards, setLatestProviderRewards] = useState<NodeProviderRewards | null>(null);
    const [isLoading, setIsLoading] = useState(true);

    useEffect(() => {
        if (provider) {
            setNodeProviderRewardsData(latestRewardRange, Principal.fromText(provider), setLatestProviderRewards, setIsLoading);
        }    
    }, [provider]);

    if (isLoading) {
        return <LoadingIndicator />;
    }

    if (!latestProviderRewards) {
        return <p>No latestNodeRewards</p>;
    }

    const xdr_conversion_rate = latestProviderRewards.xdr_conversion_rate;
    const rewards_xdr_old = latestProviderRewards.rewards_xdr_old;

    if (xdr_conversion_rate.length == 0 || rewards_xdr_old.length == 0) {
        return <p>No latestNodeRewards</p>;
    }
    const distribution_date = new Date(Number(latestProviderRewards.ts_distribution) * 1000);
    const rows: GridRowsProp = latestProviderRewards.computation_log.map((data, index) => {
        return { 
            id: index,
            col0: index,
            col1: data,
            };
    });
    const colDef: GridColDef[] = [
        { field: 'col0', headerName: 'Step', width: 100},
        { field: 'col1', headerName: 'Description', width: 1500},
        ];

    return (
        <>
            <Grid item xs={12} md={6}>
            <Box sx={boxStyleWidget('left')}>
                <WidgetNumber value={distribution_date.toDateString()} title="Date" sxPaper={{ width: '300px' }}/>
                <WidgetNumber value={(Number(latestProviderRewards.rewards_xdr_permyriad_no_reduction) / 10000).toString()} title="Expected Rewards XDR No Reduction" sxPaper={{ width: '300px' }}/>
                <WidgetNumber value={(Number(latestProviderRewards.rewards_xdr_permyriad) / 10000).toString()} title="Expected Rewards XDR"/>
                <WidgetNumber value={(Number(latestProviderRewards.xdr_conversion_rate) / 10000).toString()} title="Conversion rate"/>
            </Box>
            </Grid>
            <Grid item xs={12} md={6}>
            <Box sx={boxStyleWidget('right')}>
                <WidgetNumber value={Math.round(Number(latestProviderRewards.rewards_xdr_permyriad) / Number(latestProviderRewards.xdr_conversion_rate)).toString()} title="Expected Rewards ICP"  sxValue={{ color: '#FFCC00' }} />
                <WidgetNumber value={Math.round(Number(rewards_xdr_old[0]) / 100000000).toString()} title="Last Rewards ICP Received"  sxValue={{ color: '#FFCC00' }} />
            </Box>
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

export default NodeProviderRewardsChart;
