import { SxProps, Theme } from '@mui/material';

export const paperStyle: SxProps<Theme> = {
  bgcolor: 'background.paper',
  borderRadius: '10px',
  p: 2,
};

export const boxStyleLeft: SxProps<Theme> = {
  display: 'flex',
  flexWrap: 'wrap',
  justifyContent: 'left',
  gap: 2,
};

export const boxStyleRight: SxProps<Theme> = {
  display: 'flex',
  flexWrap: 'wrap',
  justifyContent: 'right',
  alignItems: 'left',
  gap: 2,
};

export const paperStyleWidget: SxProps<Theme> = {
    p: 2, 
    display: 'flex', 
    flexDirection: 'row', 
    alignItems: 'right', 
    width: '250px', 
    bgcolor: 'background.paper', 
    borderRadius: '10px'
  };
