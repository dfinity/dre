import React, { useEffect, useMemo, useState } from 'react';
import { Box, Grid, Paper, Typography } from '@mui/material';
import Divider from '@mui/material/Divider';
import { Link, useParams } from 'react-router-dom';
import { getDateRange } from '../utils/utils';
import FilterBar, { PeriodFilter } from './FilterBar';
import { NodeMetadata } from '../../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did';
import InfoFormatter from './NodeInfo';
import { NodeProviderChart } from './NodeProviderChart';
import { NodeProviderRewardsChart } from './NodeProviderRewards';
import { paperStyle } from '../Styles';

export interface NodeProviderPageProps {
    nodeMetadata: NodeMetadata[]
  }

export const NodeProviderPage: React.FC<NodeProviderPageProps> = ({ nodeMetadata: nodeProvidersMapping }) => {
    const { provider } = useParams();
    const { dateStart, dateEnd } = useMemo(() => getDateRange(), []);
    const [periodFilter, setPeriodFilter] = useState<PeriodFilter>({ dateStart, dateEnd });

    if (!provider) {
        return <p>No Node provider</p>;
    }

    const nodes = nodeProvidersMapping.filter(map => map.node_metadata_stored.node_provider_id.toText() == provider);
    const nodeIds = nodes.map(nodes => nodes.node_id);
    const providerName = nodes[0].node_metadata_stored.node_provider_name[0];

    if (!nodes) {
        return <p>No Node Ids found</p>;
    }

    useEffect(() => {
        window.scrollTo(0, 0);
    }, [provider]);

    return (
    <Box sx={{ p: 3 }}>
        <Paper sx={paperStyle}>
            <Grid container spacing={3}>
                <Grid item xs={12} md={12}>
                    <Typography gutterBottom variant="h5" component="div">
                        {"Node Provider"}
                    </Typography>
                    <Divider/>
                </Grid>
                <Grid item xs={12}>
                <InfoFormatter name={"Provider ID"} value={provider ? provider : "Anonym"} />
                <InfoFormatter name={"Provider Name"} value={providerName ? providerName : "Anonym"} />
                </Grid>
                <Grid item xs={12}>
                    <Typography gutterBottom variant="subtitle1" component="div">
                        Node Machines
                    </Typography>
                    {nodes.map((map, index) => (
                        <Typography key={index} gutterBottom variant="subtitle2" sx={{ color: 'text.disabled' }} component="div">
                        <Link to={`/nodes/${map.node_id.toText()}`} className="custom-link">
                            {map.node_id.toText()}
                        </Link>
                        </Typography>
                    ))} 
                    
                </Grid>
                <Grid item xs={12}>
                    <Typography variant="h6" component="div" >
                        Last Rewarding Period
                    </Typography>
                    <Divider/>
                </Grid>
                <NodeProviderRewardsChart provider={provider} />
                <Grid item xs={12}>
                <Typography variant="h6" component="div">
                    Daily Failure Rate
                </Typography>
                <Typography variant="subtitle2" sx={{ color: 'text.disabled' }} component="div">
                    Top 3 nodes with highest average failure rate in the period
                </Typography>
                <FilterBar filters={periodFilter} setFilters={setPeriodFilter} />   
                </Grid>
                <NodeProviderChart nodeIds={nodeIds} periodFilter={periodFilter}/>
            </Grid>
        </Paper>
    </Box>
    );
};
