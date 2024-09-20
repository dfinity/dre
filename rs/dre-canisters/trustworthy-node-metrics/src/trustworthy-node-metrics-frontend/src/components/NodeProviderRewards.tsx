import React, { useEffect, useState } from 'react';
import { getLatestRewardRange, LoadingIndicator, setNodeProviderRewardsData } from '../utils/utils';
import { Box, Grid } from '@mui/material';
import { Principal } from '@dfinity/principal';
import { NodeProviderRewards } from '../../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did';
import { WidgetNumber } from './Widgets';
import { boxStyleWidget } from '../Styles';

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

    return (
        <Grid item xs={12}>
            <Box sx={boxStyleWidget('right')}>
                <WidgetNumber value={Math.round(latestProviderRewards.rewards_xdr).toString()} title="Rewards XDR"  sxValue={{ color: '#FFCC00' }} />
            </Box>
        </Grid>
    );
};

export default NodeProviderRewardsChart;
