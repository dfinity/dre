import { SxProps, Theme } from '@mui/material';

export const paperStyle: SxProps<Theme> = {
  bgcolor: 'background.paper',
  borderRadius: '10px',
  p: 3,
};

export const boxStyleWidget = (justifyContent: 'left' | 'right', alignItems?: 'flex-start' | 'center' | 'flex-end'): SxProps<Theme> => ({
  display: 'flex',
  flexWrap: 'wrap',
  justifyContent: justifyContent,
  alignItems: alignItems || 'stretch',
  gap: 2,
});

export const paperStyleWidget: SxProps<Theme> = {
    p: 2, 
    display: 'flex', 
    flexDirection: 'row', 
    alignItems: 'right', 
    width: '250px', 
    bgcolor: 'background.paper', 
    borderRadius: '10px'
  };
