import React, { useState } from 'react';
import { Box, Paper, Typography } from '@mui/material';
import { axisClasses, BarChart, StackOrderType } from '@mui/x-charts';
import Divider from '@mui/material/Divider';
import { useParams } from 'react-router-dom';
import { generateChartData, getFormattedDates } from '../utils/utils';
import { PeriodFilter } from './FilterBar';
import { Root } from './NodeList';
import { NodeRewardsResponse } from '../../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did';

export interface NodeProviderPageProps {
    nodeRewards: NodeRewardsResponse[],
    periodFilter: PeriodFilter
  }

export const NodeProviderPage: React.FC<NodeProviderPageProps> = ({ nodeRewards, periodFilter }) => {
    const [stackOrder] = useState<StackOrderType>('ascending');
    const { provider } = useParams();
    const providerNodeMetrics = nodeRewards
        .filter((nodeMetrics) => nodeMetrics.node_provider_id.toText() === provider)
    const chartData = providerNodeMetrics
        .map(nodeMetrics => {
            return {
                data: generateChartData(periodFilter, nodeMetrics.daily_node_metrics),
                label: provider,
              }
    });

    console.info(chartData)
    const series = [{ ...chartData[0], stackOrder }, ...chartData.slice(1)];

    return (
        <Root>
            <Paper sx={{ p: 2, backgroundColor: '#11171E', borderRadius: '10px', color: 'white' }}>
                <Box sx={{ p: 2 }}>
                        <Typography gutterBottom variant="h6" component="div">
                            Node Provider: {provider}
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
                </Box>
            </Paper>
        </Root>
    );
};
