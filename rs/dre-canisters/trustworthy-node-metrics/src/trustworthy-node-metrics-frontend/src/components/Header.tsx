import { IconButton, Toolbar, Typography, styled, useMediaQuery, useTheme } from '@mui/material';
import React from 'react';
import { Menu as MenuIcon } from '@mui/icons-material';

const Title = styled(Typography)(() => ({
    flexGrow: 1,
    fontWeight: 500,
    fontSize: '1.5rem',
    letterSpacing: '1px',
    fontFamily: 'Roboto, sans-serif',
    color: 'white'
  }));

interface HeaderProps {
  onDrawerToggle: () => void;
}

const Header: React.FC<HeaderProps> = ({ onDrawerToggle }) =>  {
  const theme = useTheme();
  const isSmallScreen = useMediaQuery(theme.breakpoints.down('sm'));
  
  return (
      <Toolbar>
        {isSmallScreen && (
          <IconButton edge="start" color="inherit" aria-label="menu" onClick={onDrawerToggle}>
            <MenuIcon />
          </IconButton>
        )}
        <Title >
          Node Provider Rewards
        </Title>
      </Toolbar>
  );
};

export default Header;
