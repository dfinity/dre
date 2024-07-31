import { Box, Typography } from '@mui/material';
import { Gauge, gaugeClasses } from '@mui/x-charts/Gauge';
import React from 'react';

export default function FailureRateArc(value: number) {
  return (
    <Box sx={{ p: 2 }}>
    <Typography variant="subtitle1" component="div">
        Failure Rate
    </Typography>
    <Gauge
        width = {80}
        height = {80}
        value= {value}
        cornerRadius="10%"
        text={
            ({ value }) => `${value}%`
         }
        sx={(theme) => ({
            [`& .${gaugeClasses.valueText}`]: {
            fontSize: 15,
            },
            [`& .${gaugeClasses.valueArc}`]: {
            fill: '#FF6347',
            },
            [`& .${gaugeClasses.referenceArc}`]: {
            fill: theme.palette.text.disabled,
            },
        })}
    />
    </Box>
  );
}

export function RewardsArc(value: number) {
  return (
    <Box sx={{ p: 2 }}>
    <Typography variant="subtitle1" component="div">
        Node Reward
    </Typography>
    <Gauge
        width = {80}
        height = {80}
        value= {value}
        cornerRadius="10%"
        text={
            ({ value }) => `${value}%`
         }
        sx={(theme) => ({
            [`& .${gaugeClasses.valueText}`]: {
            fontSize: 15,
            },
            [`& .${gaugeClasses.valueArc}`]: {
            fill: '#52b202',
            },
            [`& .${gaugeClasses.referenceArc}`]: {
            fill: theme.palette.text.disabled,
            },
        })}
    />
    </Box>
  );
}
