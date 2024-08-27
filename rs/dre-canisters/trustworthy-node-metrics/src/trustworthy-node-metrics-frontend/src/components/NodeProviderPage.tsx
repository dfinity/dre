import React, { useState } from 'react';
import { Box, Grid, Paper, Typography } from '@mui/material';
import { axisClasses, BarChart, StackOrderType } from '@mui/x-charts';
import Divider from '@mui/material/Divider';
import { useParams } from 'react-router-dom';
import { formatDateToUTC, generateChartData, getFormattedDates } from '../utils/utils';
import { PeriodFilter } from './FilterBar';
import { Root } from './NodeList';
import { NodeRewardsResponse } from '../../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did';
import { paperStyle } from '../Styles';
import InfoFormatter from './NodeInfo';
import { ExportTable } from './ExportTable';
import { GridColDef, GridRowsProp } from '@mui/x-data-grid';

export interface NodeProviderPageProps {
    nodeRewards: NodeRewardsResponse[],
    periodFilter: PeriodFilter
  }

export const NodeProviderPage: React.FC<NodeProviderPageProps> = ({ nodeRewards, periodFilter }) => {
    const { provider } = useParams();
    const providerNodeMetrics = nodeRewards
        .filter((nodeMetrics) => nodeMetrics.node_provider_id.toText() === provider)
    const highFailureRateChart = providerNodeMetrics
        .filter(nodeMetrics => nodeMetrics.rewards_stats.rewards_reduction > 0)
        .flatMap(nodeMetrics => {
            const chartData = generateChartData(periodFilter, nodeMetrics.daily_node_metrics);
            return {
                data: chartData.map(data => data.dailyNodeMetrics? data.dailyNodeMetrics.failure_rate * 100: null),
                label: nodeMetrics.node_id.toText(),
                stack: 'total' 
            }
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
                <Grid item xs={12} md={12}>
                    <InfoFormatter name={"Provider ID"} value={provider ? provider : "Anonym"} />
                </Grid>
                <Grid item xs={12}>
                <Typography variant="h6" component="div">
                    Daily Failure Rate
                </Typography>
                <Typography variant="subtitle2" sx={{ color: 'text.disabled' }} component="div">
                    For nodes with rewards reduction
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
