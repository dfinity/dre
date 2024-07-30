import React from 'react';
import {
  Drawer as MUIDrawer,
  List,
  ListItemButton,
  ListItemText,
  Collapse,
  IconButton,
  ListItem,
  Toolbar,
} from '@mui/material';
import { Link } from 'react-router-dom';
import Logo from '../assets/icp_logo.svg'; 
import { ExpandLess, ExpandMore } from '@mui/icons-material';

interface DrawerProps {
  subnets: Set<string>;
  nodeProviders: Set<string>;
  drawerWidth: number;
}

const Drawer: React.FC<DrawerProps> = ({ subnets, nodeProviders, drawerWidth }) => {
  const [openSubnets, setOpenSubnets] = React.useState(false);
  const [openNP, setOpenNp] = React.useState(false);

  const renderCollapsibleList = (
    title: string,
    items: Set<string>,
    open: boolean,
    setOpen: React.Dispatch<React.SetStateAction<boolean>>,
    basePath: string
  ) => (
    <>
      <ListItemButton onClick={() => setOpen(!open)}>
        <ListItemText primary={title} />
        {open ? <ExpandLess /> : <ExpandMore />}
      </ListItemButton>
      <Collapse in={open} timeout="auto" unmountOnExit>
        <List component="div" disablePadding>
          {Array.from(items).map((item, index) => (
            <Link key={index} to={`/${basePath}/${item}`} style={{ textDecoration: 'none', color: 'inherit' }}>
              <ListItemButton>
                <ListItemText primary={item.toString().split("-")[0]} />
              </ListItemButton>
            </Link>
          ))}
        </List>
      </Collapse>
    </>
  );

  return (
    <MUIDrawer
      variant="permanent"
      sx={{
        width: drawerWidth,
        flexShrink: 0,
      }}
    >
      <List
        sx={{ width: '100%', maxWidth: 360, bgcolor: 'background.paper' }}
        component="nav"
        aria-labelledby="nested-list-subheader"
      >
        <ListItem>
            <IconButton edge="start" color="inherit" aria-label="logo">
              <img src={Logo} alt="Logo" style={{ height: 30 }} />
            </IconButton>
        </ListItem>
        <Toolbar />
        <Link to="/nodes"  style={{ textDecoration: 'none', color: 'inherit' }}>
          <ListItemButton>
            <ListItemText primary="Nodes" />
          </ListItemButton>
        </Link>
        {renderCollapsibleList("Subnets", subnets, openSubnets, setOpenSubnets, "subnets")}
        {renderCollapsibleList("Node Providers", nodeProviders, openNP, setOpenNp, "node-providers")}
      </List>
    </MUIDrawer>
  );
};

export default Drawer;
