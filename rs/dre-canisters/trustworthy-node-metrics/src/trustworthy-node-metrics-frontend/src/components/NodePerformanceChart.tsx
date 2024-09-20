import React, { useEffect, useState } from 'react';
import { ChartData, formatDateToUTC, generateChartData, LoadingIndicator, NodeMetricsStats, setNodeRewardsData } from '../utils/utils';
import { PeriodFilter } from './FilterBar';
import { Box, Grid } from '@mui/material';
import PerformanceChart from './PerformanceChart';
import { NodeRewards } from '../../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did';
import { ExportTable } from './ExportTable';
import { GridColDef, GridRowsProp } from '@mui/x-data-grid';
import { Principal } from '@dfinity/principal';
import { boxStyleWidget } from '../Styles';
import { WidgetNumber } from './Widgets';

export interface NodePerformanceChartProps {
    node: string;
    periodFilter: PeriodFilter;
  }

export const NodePerformanceChart: React.FC<NodePerformanceChartProps> = ({ node, periodFilter }) => {
    const [performanceData, setPerformanceData] = useState<NodeRewards | null>(null);
    const [isLoading, setIsLoading] = useState(true);

    useEffect(() => {
        if (node) {
            setNodeRewardsData(periodFilter, Principal.fromText(node), setPerformanceData, setIsLoading);
        }    
    }, [node, periodFilter]);
    
    if (isLoading) {
        return <LoadingIndicator />;
    }
    if (!performanceData) {
        return <p>No metrics for the time period selected</p>;
    }
    if (!node) {
        return <p>No metrics for the time period selected</p>;
    }

    const performanceDailyData: ChartData[] = generateChartData(periodFilter, performanceData.daily_node_metrics);
    const failureRateAvg = Math.round(performanceData.rewards_computation.failure_rate * 100);

    const rows: GridRowsProp = performanceData.daily_node_metrics.map((data, index) => {
        return { 
            id: index + 1,
            col1: new Date(Number(data.ts) / 1000000), 
            col2: node,
            col3: Number(data.num_blocks_proposed), 
            col4: Number(data.num_blocks_failed),
            col5: data.failure_rate,
            col6: data.subnet_assigned.toText(),
            };
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
        <>
            <Grid item xs={12} md={6}>
                <NodeMetricsStats stats={performanceData.rewards_computation} />
            </Grid>
            <Grid item xs={12} md={6}>
                <Box sx={boxStyleWidget('right')}>
                    <WidgetNumber value={failureRateAvg.toString().concat("%")} title="Average Failure Rate" />
                </Box>
            </Grid>
            <Grid item xs={12} md={12}>
                <PerformanceChart chartDailyData={performanceDailyData} />
            </Grid>
            <Grid item xs={12} md={12}>
                <ExportTable colDef={colDef} rows={rows}/>
            </Grid>
        </>
    );
};

export default NodePerformanceChart;
