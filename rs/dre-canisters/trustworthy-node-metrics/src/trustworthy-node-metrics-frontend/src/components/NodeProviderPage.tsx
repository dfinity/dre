import React, { useEffect, useState } from 'react';
import { Box, Grid, Paper, Typography } from '@mui/material';
import { axisClasses, BarChart, StackOrderType } from '@mui/x-charts';
import Divider from '@mui/material/Divider';
import { Link, useParams } from 'react-router-dom';
import { formatDateToUTC, generateChartData, getFormattedDates, LoadingIndicator, setNodeRewardsData } from '../utils/utils';
import { PeriodFilter } from './FilterBar';
import { Root } from './NodeList';
import { NodeProviderMapping, NodeRewardsResponse } from '../../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did';
import { paperStyle } from '../Styles';
import InfoFormatter from './NodeInfo';
import { ExportTable } from './ExportTable';
import { GridColDef, GridRowsProp } from '@mui/x-data-grid';
import { Principal } from '@dfinity/principal';

export interface NodeProviderPageProps {
    nodeProvidersMapping: NodeProviderMapping[]
    periodFilter: PeriodFilter
  }

export const NodeProviderPage: React.FC<NodeProviderPageProps> = ({ nodeProvidersMapping, periodFilter }) => {
    const { provider } = useParams();

    const [providerNodeMetrics, setProviderNodeMetrics] = useState<NodeRewardsResponse[]>([]);
    const [isLoading, setIsLoading] = useState(true);

    useEffect(() => {
        if (provider) {
            setNodeRewardsData(periodFilter, [], [Principal.fromText(provider)], setProviderNodeMetrics, setIsLoading);
        }    
    }, [periodFilter, provider]);
    
    if (isLoading) {
        return <LoadingIndicator />;
    }

    if (providerNodeMetrics.length == 0) {
        return <p>No metrics for the time period selected</p>;
    }

    const nodeIds = providerNodeMetrics
        .map((metrics) => metrics.node_id.toText());  
    const nodeIdsWithPenalty = providerNodeMetrics
        .filter(metrics => metrics.rewards_computation.rewards_reduction > 0)
        .map((metrics) => metrics.node_id.toText());  
    const highFailureRateChart = providerNodeMetrics
        .sort((a, b) => b.rewards_computation.failure_rate - a.rewards_computation.failure_rate) // Sort in descending order
        .slice(0, 3)
        .flatMap(nodeMetrics => {
            const chartData = generateChartData(periodFilter, nodeMetrics.daily_node_metrics);
            return {
                data: chartData.map(data => data.dailyNodeMetrics ? data.dailyNodeMetrics.failure_rate * 100 : null),
                label: nodeMetrics.node_id.toText(),
                stack: 'total'
            };
        });
    

    let index = 0;
    const rows: GridRowsProp = providerNodeMetrics.flatMap((nodeRewards) => {
        return nodeRewards.daily_node_metrics.map((data) => {
            index = index + 1;
            return { 
                id: index,
                col1: new Date(Number(data.ts) / 1000000), 
                col2: nodeRewards.node_id.toText(),
                col3: Number(data.num_blocks_proposed), 
                col4: Number(data.num_blocks_failed),
                col5: data.failure_rate,
                col6: data.subnet_assigned.toText(),
              };
        })
      });
      
    const colDef: GridColDef[] = [
        { field: 'col1', headerName: 'Date (UTC)', width: 200, valueFormatter: (value: Date) => formatDateToUTC(value)},
        { field: 'col2', headerName: 'Node ID', width: 550 },
        { field: 'col3', headerName: 'Blocks Proposed', width: 150 },
        { field: 'col4', headerName: 'Blocks Failed', width: 150 },
        { field: 'col5', headerName: 'Daily Failure Rate', width: 350 , valueFormatter: (value: number) => `${value * 100}%`,},
        { field: 'col6', headerName: 'Subnet Assigned', width: 550 },
        ];

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
                </Grid>
                <Grid item xs={12} md={6}>
                    <Typography gutterBottom variant="subtitle1" component="div">
                        Node Machines
                    </Typography>
                    {nodeIds.map((nodeId, index) => (
                        <Typography key={index} gutterBottom variant="subtitle2" sx={{ color: 'text.disabled' }} component="div">
                        <Link to={`/nodes/${nodeId}`} className="custom-link">
                            {nodeId}
                        </Link>
                        </Typography>
                    ))}
                    
                </Grid>
                <Grid item xs={12} md={6}>
                    
                <Divider orientation="vertical" flexItem sx={{ mx: 2 }} />
                    <Typography gutterBottom variant="subtitle1" component="div">
                        Node Machines With Penalty
                    </Typography>
                    {nodeIdsWithPenalty.length == 0 ? 
                        <Typography key={index} gutterBottom variant="subtitle2" sx={{ color: 'text.disabled' }} component="div"> None </Typography> : 
                        nodeIdsWithPenalty.map((nodeId, index) => (
                            <Typography key={index} gutterBottom variant="subtitle2" sx={{ color: 'text.disabled' }} component="div">
                            <Link to={`/nodes/${nodeId}`} className="custom-link">
                                {nodeId}
                            </Link>
                            </Typography>
                    ))}
                    <Divider orientation="vertical" flexItem sx={{ mx: 2 }} />
                </Grid>
                
                <Grid item xs={12}>
                <Typography variant="h6" component="div">
                    Daily Failure Rate
                </Typography>
                <Typography variant="subtitle2" sx={{ color: 'text.disabled' }} component="div">
                    Top 3 nodes with highest average failure rate in the period
                </Typography>
                </Grid>
                <Grid item xs={12} md={12}>
                <BarChart
                        slotProps={{ legend: { hidden: true } }}
                        xAxis={[{ 
                            scaleType: 'band',
                            data: getFormattedDates(periodFilter),
                        }]}
                        yAxis={[{
                            valueFormatter: (value: number) => `${value}%`,
                          }]}
                        leftAxis={null}
                        borderRadius={9}
                        series={highFailureRateChart}
                        height={300}
                    />
                </Grid>
                <Grid item xs={12} md={12}>
                    <ExportTable colDef={colDef} rows={rows}/>
                </Grid>
            </Grid>
        </Paper>
    </Box>
    );
};
