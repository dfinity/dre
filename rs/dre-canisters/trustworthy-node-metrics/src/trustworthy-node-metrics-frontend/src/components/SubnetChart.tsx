import React, { useState } from 'react';
import { Box, Paper, Typography } from '@mui/material';
import { axisClasses, BarChart, StackOrderType } from '@mui/x-charts';
import Divider from '@mui/material/Divider';
import { useParams } from 'react-router-dom';
import { generateChartData, getFormattedDates } from '../utils/utils';
import { ChartsNoDataOverlay } from '@mui/x-charts/ChartsOverlay';
import RewardTable from './RewardTable';
import { DashboardNodeMetrics } from '../models/NodeMetrics';
import { PeriodFilter } from './FilterBar';
import { Root } from './NodeList';

export interface SubnetChartProps {
    dashboardNodeMetrics: DashboardNodeMetrics[],
    periodFilter: PeriodFilter
  }

export const SubnetChart: React.FC<SubnetChartProps> = ({ dashboardNodeMetrics, periodFilter }) => {
    const [stackOrder] = useState<StackOrderType>('ascending');
    const { subnet } = useParams();
    const subnetNodeMetrics = dashboardNodeMetrics
        .filter((nodeMetrics) => nodeMetrics.dailyData.some((daily) => daily.subnetId === subnet))
    const chartData = subnetNodeMetrics
        .filter((nodeMetrics) => nodeMetrics.dailyData.some(fr => fr.failureRate >= 10))
        .map(nodeMetrics => {
            return {
                data: generateChartData(periodFilter, nodeMetrics.dailyData).map(daily => daily.failureRate),
                label: nodeMetrics.nodeId,
              }
    });
    const series = [{ ...chartData[0], stackOrder }, ...chartData.slice(1)];

    return (
        <Root>
            <Paper sx={{ p: 2, backgroundColor: '#11171E', borderRadius: '10px', color: 'white' }}>
                <Box sx={{ p: 2 }}>
                        <Typography gutterBottom variant="h6" component="div">
                            Subnet: {subnet}
                        </Typography>
                </Box>
                <Divider />
                <Box sx={{ p: 3 }}>
                    <Divider style={{ fontSize: '17px' }}>Daily Failure Rate</Divider>
                    {chartData.length > 0 ? (
                    <BarChart
                        slotProps={{ legend: { hidden: true } }}
                        xAxis={[{ 
                            scaleType: 'band',
                            data: getFormattedDates(periodFilter),
                        }]}
                        yAxis={[
                            {
                                valueFormatter: value => `${value}%`,
                                label: 'Failure Rate',
                                min: 0,
                                max: 100,
                            },
                        ]}
                        sx={{
                            p: 2,
                            [`.${axisClasses.left} .${axisClasses.label}`]: {
                                transform: 'translateX(-25px)',
                            },
                        }}
                        borderRadius={9}
                        series={series}
                        height={500}
                    />) : (<ChartsNoDataOverlay/>)}
                    <Box sx={{ p: 10 }}>
                    <RewardTable dashboardNodeMetrics={subnetNodeMetrics}/>

                    </Box>
                </Box>
            </Paper>
        </Root>
    );
};