import React from 'react';
import {
  Drawer as MUIDrawer,
  List,
  Typography,
  MenuItem,
  Theme,
  Accordion,
  AccordionSummary,
  AccordionDetails,
  FormControl,
  InputLabel,
  Select,
  ListItem,
  IconButton,
  Toolbar,
  Button,
  ListItemButton,
} from '@mui/material';
import { Link } from 'react-router-dom';
import ExpandMoreIcon from '@mui/icons-material/ExpandMore';
import { Filters } from './FilterBar';
import { SxProps } from '@mui/system';
import Logo from '../assets/icp_logo.svg'; 

interface DrawerProps {
  subnets: Set<string>;
  nodeProviders: Set<string>;
  drawerWidth: number;
  theme: Theme;
  setFilters: React.Dispatch<React.SetStateAction<Filters>>;
}

const Drawer: React.FC<DrawerProps> = ({ subnets, nodeProviders, drawerWidth, theme, setFilters }) => {

  const handleSubnetSelect = (subnet: string) => {
    setFilters((prev) => ({ ...prev, subnet, nodeProvider: null }));
  };

  const handleNodeProviderSelect = (nodeProvider: string) => {
    setFilters((prev) => ({ ...prev, nodeProvider, subnet: null  }));
  };

  return (
    <MUIDrawer
    variant="permanent"
    sx={{
        width: drawerWidth,
        flexShrink: 0,
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
        <ListItem>
        <Button>
        <Link to="/nodes" style={{ textDecoration: 'none', color: 'inherit' }}>
                Nodes
        </Link>
        </Button>
        <Button>
        </Button>
        </ListItem>
        <Accordion>
        <AccordionSummary
            expandIcon={<ExpandMoreIcon style={{ color: theme.palette.common.white }} />}
            aria-controls="panel1a-content"
            id="panel1a-header"
        >
            <Typography>Subnets</Typography>
        </AccordionSummary>
        <AccordionDetails>
            <FormControl fullWidth variant="outlined">

            <List>
                {Array.from(subnets).map((subnet, index) => (
                <Link key={index} to="/subnets"   style={{ textDecoration: 'none', color: 'inherit' }}>
                <ListItem disablePadding>
                <ListItemButton onClick={() => handleSubnetSelect(subnet)}>
                {subnet.toString().split("-")[0]}
                </ListItemButton>
                </ListItem>
                </Link>
                ))}
            </List>
            </FormControl>
        </AccordionDetails>
        </Accordion>
        <Accordion>
        <AccordionSummary
            expandIcon={<ExpandMoreIcon style={{ color: theme.palette.common.white }} />}
            aria-controls="panel1a-content"
            id="panel1a-header"
        >
            <Typography>Node Providers</Typography>
        </AccordionSummary>
        <AccordionDetails>
            <FormControl fullWidth variant="outlined">
                {Array.from(nodeProviders).map((nodeProvider, index) => (
                <MenuItem key={index} onClick={() => handleNodeProviderSelect(nodeProvider)}>
                    <Link to="/node-providers" style={{ textDecoration: 'none', color: 'inherit' }}>
                    {nodeProvider.toString().split("-")[0]}
                    </Link>
                </MenuItem>
                ))}
            </FormControl>
        </AccordionDetails>
        </Accordion>
    </List>
    </MUIDrawer>
  );
};

export default Drawer;


