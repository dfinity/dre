import { Toolbar, Typography, styled } from '@mui/material';
import React from 'react';

const Title = styled(Typography)(() => ({
    flexGrow: 1,
    fontWeight: 500,
    fontSize: '1.5rem',
    letterSpacing: '1px',
    fontFamily: 'Roboto, sans-serif',
    color: 'white'
  }));


const Header = () => {
  return (
      <Toolbar>
        <Title >
          Node Rewards
        </Title>
      </Toolbar>
  );
};

export default Header;
