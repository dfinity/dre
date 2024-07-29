import React from 'react';
import { Line } from 'react-chartjs-2';
import { Box } from '@mui/material';
import { NodeMetrics } from '../models/NodeMetrics';


interface ChartProps {
    data: NodeMetrics[];
    name: string | null;
  }


const ChartComponent: React.FC<ChartProps>  = ({ data }) => {
  return (
    <Box width="75%">
    </Box>
  );
};

export default ChartComponent;
