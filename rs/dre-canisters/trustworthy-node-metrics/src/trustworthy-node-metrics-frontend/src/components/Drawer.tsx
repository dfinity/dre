import React, { useState } from 'react';
import { Drawer, List, ListItem, Menu, MenuItem, Typography } from '@mui/material';

const SidebarDrawer = ({ drawerWidth, filters, setFilters, subnets }) => {
  const [menuAnchorEl, setMenuAnchorEl] = useState<null | HTMLElement>(null); // Menu anchor state

  const handleMenuOpen = (event: React.MouseEvent<HTMLElement>) => {
    setMenuAnchorEl(event.currentTarget);
  };

  const handleMenuClose = () => {
    setMenuAnchorEl(null);
  };

  const handleSubnetSelect = (subnet: string) => {
    setFilters((prev) => ({ ...prev, subnet }));
    handleMenuClose();
  };

  return (
    <>
      <Drawer
        variant="permanent"
        sx={{ width: drawerWidth, flexShrink: 0 }}
        PaperProps={{
          sx: {
            width: drawerWidth,
            bgcolor: 'primary.main', // Set the background color to the primary color
            color: 'white', // Set the text color to white for better visibility
          }
        }}
      >
        <List>
          <ListItem button onClick={handleMenuOpen}>
            <Typography>Subnet</Typography>
          </ListItem>
        </List>
      </Drawer>
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

export default SidebarDrawer;
