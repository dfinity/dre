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
  nodeProviders: Set<string>;
  drawerWidth: number;
  open: boolean;
  onClose: () => void;
}

const Drawer: React.FC<DrawerProps> = ({ subnets, nodeProviders, drawerWidth, open, onClose }) => {
  const [isSubnetsOpen, setIsSubnetsOpen] = React.useState(false);
  const [isNodeProvidersOpen, setIsNodeProvidersOpen] = React.useState(false);

  const theme = useTheme();
  const isSmallScreen = useMediaQuery(theme.breakpoints.down('sm'));

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
        variant={isSmallScreen ? 'temporary' : 'permanent'}
        open={open}
        onClose={onClose}
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
          {renderCollapsibleList("Node Providers", nodeProviders, isNodeProvidersOpen, setIsNodeProvidersOpen, "node-providers")}
        </List>
      </MUIDrawer>
  );
};

export default Drawer;
