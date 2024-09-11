import React, { useState, useEffect, useMemo } from 'react';
import { Box, CircularProgress, CssBaseline, ThemeProvider, createTheme, useMediaQuery, useTheme } from '@mui/material';
import FilterBar, { PeriodFilter } from './components/FilterBar';
import Drawer from './components/Drawer'; 
import { BrowserRouter as Router, Route, Routes, Navigate } from 'react-router-dom';
import { trustworthy_node_metrics } from '../../declarations/trustworthy-node-metrics/index.js';
import { NodeList } from './components/NodeList';
import Header from './components/Header';
import { getDateRange, LoadingIndicator } from './utils/utils';
import { NodePage } from './components/NodePage';
import { NodeProviderPage } from './components/NodeProviderPage';
import { NodeProviderMapping } from '../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did.d';

// Theme configuration
const darkTheme = createTheme({
  palette: {
    mode: 'dark',
    primary: {
      main: '#3f51b5',
    },
    secondary: {
      main: '#f50057',
    },
  },
});

const App: React.FC = () => {
  const { dateStart, dateEnd } = useMemo(() => getDateRange(), []);
  
  const [periodFilter, setPeriodFilter] = useState<PeriodFilter>({ dateStart, dateEnd });
  const [providers, setProviders] = useState<Set<string>>(new Set());
  const [nodeProvidersMapping, setNodeProvidersMapping] = useState<NodeProviderMapping[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const theme = useTheme();
  const isSmallScreen = useMediaQuery(theme.breakpoints.down('sm'));
  const drawerWidth = 180;

  useEffect(() => {
    const updateRewards = async () => {
      try {
        setIsLoading(true);
        const nodeProviderMapping = await trustworthy_node_metrics.node_provider_mapping();
        const providers = new Set(nodeProviderMapping.flatMap(node => node.node_provider_id.toText()));

        setProviders(providers);
        setNodeProvidersMapping(nodeProviderMapping);
      } catch (error) {
        console.error("Error fetching nodeProviderMapping:", error);
      } finally {
        setIsLoading(false);
      }
    };
    
    updateRewards();
  }, [periodFilter]);

  const drawerProps = useMemo(() => ({
    providers,
    drawerWidth,
    temporary: isSmallScreen,
    drawerOpen,
    onClosed: () => setDrawerOpen(false)
  }), [providers, drawerWidth, isSmallScreen, drawerOpen]);

  return (
    <ThemeProvider theme={darkTheme}>
      <CssBaseline />
      <Router>
        <Box sx={{ display: 'flex' }}>
          <Drawer {...drawerProps} />
          <Box sx={{ flexGrow: 1, width: `calc(100% - ${drawerWidth}px)` }}>
            <Header withDrawerIcon={isSmallScreen} onDrawerIconClicked={() => setDrawerOpen(true)} />
            <FilterBar filters={periodFilter} setFilters={setPeriodFilter} />
            
            <Routes>
              <Route path="/" element={<Navigate to="/nodes" replace />} />
              <Route path="/nodes" element={
                isLoading ? <LoadingIndicator /> : <NodeList nodeProviderMapping={nodeProvidersMapping} periodFilter={periodFilter} />
              } />
              <Route path="/nodes/:node" element={
                isLoading ? <LoadingIndicator /> : <NodePage periodFilter={periodFilter} />
              } />
              <Route path="/providers/:provider" element={
                isLoading ? <LoadingIndicator /> : <NodeProviderPage periodFilter={periodFilter} />
              } />
            </Routes>
          </Box>
        </Box>
      </Router>
    </ThemeProvider>
  );
}

export default App;
