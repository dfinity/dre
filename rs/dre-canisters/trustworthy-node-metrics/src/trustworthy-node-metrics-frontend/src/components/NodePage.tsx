import React, { useEffect, useMemo, useState } from 'react';
import { getDateRange } from '../utils/utils';
import FilterBar, { PeriodFilter } from './FilterBar';
import { Box, Divider, Grid, Paper, Typography } from '@mui/material';
import { Link, useParams } from 'react-router-dom';
import { paperStyle } from '../Styles';
import { NodeMetadata } from '../../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did';
import InfoFormatter from './NodeInfo';
import NodeRewardsChart from './NodeRewardsChart';
import NodePerformanceChart from './NodePerformanceChart';

export interface NodeProviderPageProps {
    nodeProvidersMapping: NodeMetadata[]
  }
  
export const NodePage: React.FC<NodeProviderPageProps> = ({ nodeProvidersMapping }) => {
    const { dateStart, dateEnd } = useMemo(() => getDateRange(), []);
    const [periodFilter, setPeriodFilter] = useState<PeriodFilter>({ dateStart, dateEnd });

    const { node } = useParams();
    if (!node) {
        return <p>No node found</p>;
    }

    const mapping = nodeProvidersMapping.find(map => map.node_id.toText() == node);

    if (!mapping) {
        return <p>No mapping found</p>;
    }

    useEffect(() => {
        window.scrollTo(0, 0);
    }, []);

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
                        <InfoFormatter name={"Node ID"} value={node} />
                        <InfoFormatter name={"Node Type"} value={mapping.node_metadata_stored.node_type} />
                        <InfoFormatter name={"Datacenter ID"} value={mapping.node_metadata_stored.dc_id} />
                        <InfoFormatter name={"Region"} value={mapping.node_metadata_stored.region} />
                        <Typography gutterBottom variant="subtitle1" component="div">
                            Node Provider ID
                        </Typography>
                        <Typography gutterBottom variant="subtitle2" sx={{ color: 'text.disabled' }} component="div">
                        <Link to={`/providers/${mapping.node_metadata_stored.node_provider_id.toText()}`} className="custom-link">
                            {mapping.node_metadata_stored.node_provider_id.toText()}
                        </Link>
                        </Typography>
                    </Grid>
                    <Grid item xs={12}>
                    <Typography variant="h6" component="div" >
                        Last Reward Metrics
                    </Typography>
                    <Divider/>
                    </Grid>
                    <NodeRewardsChart node={node}/>
                    <Grid item xs={12} md={12}>
                        <Typography variant="h6" component="div" >
                            Daily Node Performance
                        </Typography>
                        <Divider/>
                        <FilterBar filters={periodFilter} setFilters={setPeriodFilter} />     
                    </Grid>
                    <NodePerformanceChart node={node} periodFilter={periodFilter}/>
                </Grid>
            </Paper>
        </Box>
    );
};

export default NodePage;
