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
  providers: Map<string, string>;
  drawerWidth: number;
  temporary: boolean;
  drawerOpen: boolean;
  onClosed: () => void;
}

const Drawer: React.FC<DrawerProps> = ({ providers, drawerWidth, temporary, drawerOpen, onClosed }) => {
  const [isNodeProvidersOpen, setIsNodeProvidersOpen] = React.useState(false);
  const [selectedIndex, setSelectedIndex] = React.useState<number | null>(null);

  const renderCollapsibleList = (
    title: string,
    items: Map<string, string>,
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
              <Link key={index} to={`/${basePath}/${item[0]}`} style={{ textDecoration: 'none', color: 'inherit' }}>
                <ListItemButton
                  key={index}
                  selected={selectedIndex === index}
                  onClick={() => setSelectedIndex(index)}
                >
                  <ListItemText primary={item[1]} />
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
          {renderCollapsibleList("Node Providers", providers, isNodeProvidersOpen, setIsNodeProvidersOpen, "providers")}
        </List>
      </MUIDrawer>
  );
};

export default Drawer;
