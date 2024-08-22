import { Box, Paper, Stack, Typography } from '@mui/material';
import { Gauge, gaugeClasses } from '@mui/x-charts/Gauge';
import { boxStyleWidget, paperStyleWidget } from '../Styles';

import React from 'react';

export function WidgetNumber({ value, title }: { value: string, title: string }) {
  return (
    <Paper 
      elevation={5} 
      sx={paperStyleWidget}
    >
      <Stack spacing={0.8}>
      <Typography variant="h4">
        {value}
      </Typography>
      <Typography variant="subtitle2" sx={{ color: 'text.disabled' }}>
        {title}
      </Typography>
      </Stack>
    </Paper>
  );
}
export function WidgetGauge({ value, title }: { value: number, title: string }) {
  return (
    <Box 
      sx={boxStyleWidget('right')}
    >
      <Paper 
        elevation={15} 
        sx={{
          p: 2, 
          display: 'flex', 
          flexDirection: 'row', 
          alignItems: 'center', 
          justifyContent: 'center',
          width: '150px',
          height: '180px',
          bgcolor: 'background.paper', 
          borderRadius: '10px'
        }}
      >
        <Stack spacing={0.8}>

        <Gauge
          width={100}
          height={100}
          value={value}
          cornerRadius="10%"
          text={({ value }) => `${value}%`}
          sx={(theme) => ({
            [`& .${gaugeClasses.valueText}`]: {
              fontSize: 20,
            },
            [`& .${gaugeClasses.valueArc}`]: {
              fill: '#52b202',
            },
            [`& .${gaugeClasses.referenceArc}`]: {
              fill: theme.palette.text.disabled,
            },
          })}
        />
        <Typography variant="subtitle1" sx={{ mb: 2 }}>
          {title}
        </Typography>
        </Stack>
      </Paper>
    </Box>
  );
}
