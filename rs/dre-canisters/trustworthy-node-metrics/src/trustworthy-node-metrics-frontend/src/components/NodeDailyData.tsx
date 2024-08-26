import * as React from 'react';
import { DataGrid, GridColDef, GridRowsProp, GridToolbarContainer, GridToolbarExport } from '@mui/x-data-grid';
import { ChartData } from '../utils/utils';

function CustomToolbar() {
  return (
    <GridToolbarContainer>
      <GridToolbarExport />
    </GridToolbarContainer>
  );
}

interface ExportCustomToolbarProps {
  chartDailyData: ChartData[];
}

export const ExportCustomToolbar: React.FC<ExportCustomToolbarProps> = ({ chartDailyData }) => {
  const rows: GridRowsProp = chartDailyData.map((dailyData, index) => {
    return { 
      id: index + 1,
      col1: dailyData.date.toLocaleDateString('en-GB'), 
      col2: dailyData.dailyNodeMetrics?.num_blocks_proposed, 
      col3: dailyData.dailyNodeMetrics?.num_blocks_failed,
      col4: dailyData.dailyNodeMetrics?.subnet_assigned,
    };
  });
  
  const columns: GridColDef[] = [
    { field: 'col1', headerName: 'Date', width: 200 },
    { field: 'col2', headerName: 'Blocks Proposed', width: 150 },
    { field: 'col3', headerName: 'Blocks Failed', width: 150 },
    { field: 'col4', headerName: 'Subnet Assigned', width: 550 },
  ];
  return (
    <div style={{ height: 700, width: '100%' }}>
      <DataGrid
        rows={rows} 
        columns={columns} 
        slots={{
          toolbar: CustomToolbar,
        }}
      />
    </div>
  );
}
