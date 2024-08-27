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
  useMediaQuery,
  useTheme,
} from '@mui/material';
import { Link } from 'react-router-dom';
import Logo from '../assets/icp_logo.svg'; 
import { ExpandLess, ExpandMore } from '@mui/icons-material';

interface DrawerProps {
  subnets: Set<string>;
  providers: Set<string>;
  drawerWidth: number;
  temporary: boolean;
  drawerOpen: boolean;
  onClosed: () => void;
}

const Drawer: React.FC<DrawerProps> = ({ subnets, providers, drawerWidth, temporary, drawerOpen, onClosed }) => {
  const [isSubnetsOpen, setIsSubnetsOpen] = React.useState(false);
  const [isNodeProvidersOpen, setIsNodeProvidersOpen] = React.useState(false);
  
  const renderCollapsibleList = (
    title: string,
    items: Set<string>,
    isOpen: boolean,
    toggleOpen: React.Dispatch<React.SetStateAction<boolean>>,
    basePath: string
  ) => {
    const itemList = items ? Array.from(items) : [];

    return (
      <>
        <ListItemButton onClick={() => toggleOpen(!isOpen)}>
          <ListItemText primary={title} />
          {isOpen ? <ExpandLess /> : <ExpandMore />}
        </ListItemButton>
        <Collapse in={isOpen} timeout="auto" unmountOnExit>
          <List component="div" disablePadding>
            {itemList.map((item, index) => (
              <Link key={index} to={`/${basePath}/${item}`} style={{ textDecoration: 'none', color: 'inherit' }}>
                <ListItemButton>
                  <ListItemText primary={item.split("-")[0]} />
                </ListItemButton>
              </Link>
            ))}
          </List>
        </Collapse>
      </>
    );
  };

  return (
      <MUIDrawer
        variant={temporary ? 'temporary' : 'permanent'}
        open={drawerOpen}
        onClose={onClosed}
        sx={{
          width: drawerWidth,
          flexShrink: 0,
          '& .MuiDrawer-paper': {
            width: drawerWidth,
            boxSizing: 'border-box',
          },
        }}
      >
        <List sx={{ width: '100%' }}>
          <ListItem>
            <IconButton edge="start" color="inherit" aria-label="logo">
              <img src={Logo} alt="Logo" style={{ height: 30 }} />
            </IconButton>
          </ListItem>
          <Toolbar />
          <Link to="/nodes" style={{ textDecoration: 'none', color: 'inherit' }}>
            <ListItemButton>
              <ListItemText primary="Nodes" />
            </ListItemButton>
          </Link>
          {renderCollapsibleList("Subnets", subnets, isSubnetsOpen, setIsSubnetsOpen, "subnets")}
          {renderCollapsibleList("Node Providers", providers, isNodeProvidersOpen, setIsNodeProvidersOpen, "providers")}
        </List>
      </MUIDrawer>
  );
};

export default Drawer;
