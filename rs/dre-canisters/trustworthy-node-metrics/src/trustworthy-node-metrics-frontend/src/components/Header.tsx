import { IconButton, Toolbar, Typography, styled} from '@mui/material';
import React from 'react';
import { Menu as MenuIcon } from '@mui/icons-material';

const Title = styled(Typography)(() => ({
    flexGrow: 1,
    fontWeight: 500,
    fontSize: '1.5rem',
    fontFamily: 'Roboto, sans-serif',
    color: 'white'
  }));

interface HeaderProps {
  withDrawerIcon: boolean;
  onDrawerIconClicked: () => void;
}

const Header: React.FC<HeaderProps> = ({ withDrawerIcon, onDrawerIconClicked }) =>  {
  return (
      <Toolbar>
        {withDrawerIcon && (
          <IconButton edge="start" color="inherit" aria-label="menu" onClick={onDrawerIconClicked}>
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
