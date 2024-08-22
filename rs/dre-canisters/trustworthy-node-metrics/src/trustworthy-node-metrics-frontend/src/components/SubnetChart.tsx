import React, { useState } from 'react';
import { Box, Paper, Typography } from '@mui/material';
import { axisClasses, BarChart, StackOrderType } from '@mui/x-charts';
import Divider from '@mui/material/Divider';
import { useParams } from 'react-router-dom';
import { generateChartData, getFormattedDates } from '../utils/utils';
import RewardTable from './RewardTable';
import { PeriodFilter } from './FilterBar';
import { Root } from './NodeList';
import { NodeRewardsResponse } from '../../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did';

export interface SubnetChartProps {
    nodeRewards: NodeRewardsResponse[],
    periodFilter: PeriodFilter
  }

export const SubnetChart: React.FC<SubnetChartProps> = ({ nodeRewards, periodFilter }) => {
    const [stackOrder] = useState<StackOrderType>('ascending');
    const { subnet } = useParams();
    const subnetNodeMetrics = nodeRewards
        .filter((nodeMetrics) => nodeMetrics.daily_node_metrics.some((daily) => daily.subnet_assigned.toText() === subnet))
    const chartData = nodeRewards
        .filter((nodeMetrics) => nodeMetrics.daily_node_metrics.some(data => data.failure_rate >= 0.1))
        .map(nodeMetrics => {
            return {
                data: generateChartData(periodFilter, nodeMetrics.daily_node_metrics).map(daily => 3),
                label: nodeMetrics.node_id.toText(),
              }
    });

    console.info(chartData)
    const series = [{ ...chartData[0], stackOrder }, ...chartData.slice(1)];

    return (
        <Root>
            <Paper sx={{ p: 2, backgroundColor: '#11171E', borderRadius: '10px'}}>
                <Box sx={{ p: 2 }}>
                        <Typography gutterBottom variant="h6" component="div">
                            Subnet: {subnet}
                        </Typography>
                </Box>
                <Divider />
                <Box sx={{ p: 3 }}>
                    <Divider style={{ fontSize: '17px' }}>Daily Failure Rate (grater 10%)</Divider>
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
                    />
                    <Box sx={{ p: 10 }}>
                    <RewardTable nodeRewards={subnetNodeMetrics}/>
                    </Box>
                </Box>
            </Paper>
        </Root>
    );
};
