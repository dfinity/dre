import React, { useEffect, useState } from 'react';
import { Grid } from '@mui/material';
import { axisClasses, BarChart } from '@mui/x-charts';
import { formatDateToUTC, generateChartData, getFormattedDates, LoadingIndicator, setNodeRewardsData } from '../utils/utils';
import { PeriodFilter } from './FilterBar';
import { NodeRewardsResponse } from '../../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did';
import { ExportTable } from './ExportTable';
import { GridColDef, GridRowsProp } from '@mui/x-data-grid';
import { Principal } from '@dfinity/principal';

export interface NodeProviderChartProps {
    provider: string,
    periodFilter: PeriodFilter
  }

export const NodeProviderChart: React.FC<NodeProviderChartProps> = ({ provider, periodFilter }) => {
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

    const highFailureRateChart = providerNodeMetrics
        .sort((a, b) => b.rewards_computation.failure_rate - a.rewards_computation.failure_rate)
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
        <>
            <Grid item xs={12} md={12}>
            <BarChart
                    slotProps={{ legend: { hidden: true } }}
                    xAxis={[{ 
                        scaleType: 'band',
                        data: getFormattedDates(periodFilter),
                    }]}
                    yAxis={[{
                        label: 'Total Failure Rate',
                        valueFormatter: (value: number) => `${value}%`,
                        }]}
                    sx={{
                    [`.${axisClasses.left} .${axisClasses.label}`]: {
                        transform: 'translate(-20px, 0)',
                    },
                    }}
                    margin={{ left: 60}}
                    borderRadius={9}
                    series={highFailureRateChart}
                    height={300}
                />
            </Grid>
            <Grid item xs={12} md={12}>
            <ExportTable colDef={colDef} rows={rows}/>
            </Grid>
        </>

    );
};