import { Box, Paper, Stack, SxProps, Typography } from '@mui/material';
import { Gauge, gaugeClasses } from '@mui/x-charts/Gauge';
import { boxStyleWidget, paperStyleWidget } from '../Styles';

import React from 'react';

export function WidgetNumber({
  value,
  title,
  sxValue = {},
  sxPaper = {}, 
}: {
  value: string;
  title: string;
  sxValue?: SxProps;
  sxPaper?: SxProps;
}) {
  const sxPaperJoin = { ...paperStyleWidget, ...sxPaper };
  return (
    <Paper elevation={5} sx={sxPaperJoin}> 
      <Stack spacing={0.8}>
        <Typography variant="h4" sx={sxValue}>
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
          width: '140px',
          height: '160px',
          bgcolor: 'background.paper', 
          borderRadius: '10px'
        }}
      >
        <Stack spacing={0.8}>
        <Typography variant="subtitle1" sx={{ mb: 2 }}>
          {title}
        </Typography>
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
        </Stack>
      </Paper>
    </Box>
  );
}
