// src/components/Drawer.tsx

import React, { useState } from 'react';
import { Drawer as MUIDrawer, List, ListItem, Typography, Menu, MenuItem, Box, Theme, IconButton, Toolbar } from '@mui/material';
import { SxProps } from '@mui/system';
import Logo from '../assets/icp_logo.svg'; // Import SVG as React component

interface DrawerProps {
  subnets: Set<string>;
  drawerWidth: number;
  theme: Theme;
  onSubnetSelect: (subnet: string) => void;
}

const Drawer: React.FC<DrawerProps> = ({ subnets, drawerWidth, theme, onSubnetSelect }) => {
  const [drawerOpen, setDrawerOpen] = useState<boolean>(false);
  const [menuAnchorEl, setMenuAnchorEl] = useState<null | HTMLElement>(null);

  const handleDrawerOpen = () => {
    setDrawerOpen(true);
  };

  const handleDrawerClose = () => {
    setDrawerOpen(false);
  };

  const handleMenuOpen = (event: React.MouseEvent<HTMLElement>) => {
    setMenuAnchorEl(event.currentTarget);
  };

  const handleMenuClose = () => {
    setMenuAnchorEl(null);
  };

  const handleSubnetSelect = (subnet: string) => {
    onSubnetSelect(subnet);
    handleMenuClose();
  };

  return (
    <>
      <MUIDrawer
        variant="permanent"
        open={drawerOpen}
        onClose={handleDrawerClose}
        sx={{
          width: drawerWidth,
          flexShrink: 0
        }}
      >
        <List>
            <ListItem>
              <IconButton edge="start" color="inherit" aria-label="logo">
                <img src={Logo} alt="Logo" style={{ height: 30 }} />
              </IconButton>
            </ListItem>
            <ListItem>
              <Toolbar />
            </ListItem>
            <ListItem button onClick={handleMenuOpen}>
              <Typography variant="h6">Subnets</Typography>
            </ListItem>
          </List>
      </MUIDrawer>
      <Menu
        anchorEl={menuAnchorEl}
        open={Boolean(menuAnchorEl)}
        onClose={handleMenuClose}
      >
        <MenuItem value="" onClick={() => handleSubnetSelect('')}>
          <em>None</em>
        </MenuItem>
        {Array.from(subnets).map((subnet, index) => (
          <MenuItem key={index} onClick={() => handleSubnetSelect(subnet)}>
            {subnet}
          </MenuItem>
        ))}
      </Menu>
    </>
  );
};

export default Drawer;
