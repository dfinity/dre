import * as React from 'react';
import { DataGrid, GridColDef, GridRowsProp, GridToolbarContainer, GridToolbarExport } from '@mui/x-data-grid';

function CustomToolbar() {
  return (
    <GridToolbarContainer>
      <GridToolbarExport />
    </GridToolbarContainer>
  );
}

interface ExportCustomToolbarProps {
  colDef: GridColDef[];
  rows: GridRowsProp;
}

export const ExportTable: React.FC<ExportCustomToolbarProps> = ({ colDef, rows }) => {
  return (
    <div style={{ height: 1000, width: '100%' }}>
      <DataGrid
        rows={rows} 
        columns={colDef} 
        slots={{
          toolbar: CustomToolbar,
        }}
      />
    </div>
  );
}
