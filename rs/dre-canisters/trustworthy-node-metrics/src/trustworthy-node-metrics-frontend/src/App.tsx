import React, { useState, useEffect, useMemo } from 'react';
import { Box, CircularProgress, CssBaseline, ThemeProvider, createTheme } from '@mui/material';
import FilterBar, { PeriodFilter } from './components/FilterBar';
import Drawer from './components/Drawer'; 
import { BrowserRouter as Router, Route, Routes, Navigate } from 'react-router-dom';
import { trustworthy_node_metrics } from '../../declarations/trustworthy-node-metrics/index.js';
import { NodeRewardsArgs, NodeRewardsResponse } from '../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did.js';
import { NodeList } from './components/NodeList';
import Header from './components/Header';
import { dateToNanoseconds } from './utils/utils';
import { NodeChart } from './components/NodePage';

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

const getDateRange = () => {
  const dateStart = new Date();
  const dateEnd = new Date();
  dateStart.setDate(dateStart.getDate() - 30);
  dateStart.setHours(0, 0, 0, 0);
  dateEnd.setHours(23, 59, 59, 999);
  return { dateStart, dateEnd };
};

const LoadingIndicator: React.FC = () => (
  <Box
    sx={{
      display: 'flex',
      justifyContent: 'center',
      alignItems: 'center',
      height: '100vh',
    }}
  >
    <CircularProgress />
  </Box>
);

const App: React.FC = () => {
  const { dateStart, dateEnd } = useMemo(() => getDateRange(), []);
  
  const [periodFilter, setPeriodFilter] = useState<PeriodFilter>({ dateStart, dateEnd });
  const [nodeRewards, setNodeRewards] = useState<NodeRewardsResponse[]>([]);
  const [subnets, setSubnets] = useState<Set<string>>(new Set());
  const [nodeProviders, setNodeProviders] = useState<Set<string>>(new Set());
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const drawerWidth = 180;

  useEffect(() => {
    const updateRewards = async () => {
      try {
        setIsLoading(true);
        const request: NodeRewardsArgs = {
          from_ts: dateToNanoseconds(periodFilter.dateStart),
          to_ts: dateToNanoseconds(periodFilter.dateEnd),
        };
        const nodeRewardsResponse = await trustworthy_node_metrics.node_rewards(request);
        const sortedNodeRewards = nodeRewardsResponse.sort((a, b) => a.rewards_percent - b.rewards_percent);
        const subnets = new Set(sortedNodeRewards.flatMap(node => node.daily_node_metrics.map(data => data.subnet_assigned.toText())));
        
        setNodeRewards(sortedNodeRewards);
        setSubnets(subnets);
      } catch (error) {
        console.error("Error fetching node:", error);
      } finally {
        setIsLoading(false);
      }
    };
    
    updateRewards();
  }, [periodFilter]);

  const handleDrawerToggle = () => setDrawerOpen(prev => !prev);

  const drawerProps = useMemo(() => ({
    subnets,
    nodeProviders,
    drawerWidth,
    open: drawerOpen,
    onClose: () => setDrawerOpen(false),
  }), [subnets, nodeProviders, drawerWidth, drawerOpen]);

  return (
    <ThemeProvider theme={darkTheme}>
      <CssBaseline />
      <Router>
        <Box sx={{ display: 'flex' }}>
          <Drawer {...drawerProps} />
          <Box sx={{ flexGrow: 1, width: `calc(100% - ${drawerWidth}px)` }}>
            <Header onDrawerToggle={handleDrawerToggle} />
            <FilterBar filters={periodFilter} setFilters={setPeriodFilter} />
            
            <Routes>
              <Route path="/" element={<Navigate to="/nodes" replace />} />
              <Route path="/nodes" element={
                isLoading ? <LoadingIndicator /> : <NodeList nodeRewards={nodeRewards} periodFilter={periodFilter} />
              } />
              <Route path="/nodes/:node" element={
                isLoading ? <LoadingIndicator /> : <NodeChart nodeRewards={nodeRewards} periodFilter={periodFilter} />
              } />
              <Route path="/subnets/:subnet" element={
                // TODO: Add subnet page
                isLoading ? <LoadingIndicator /> : <NodeList nodeRewards={nodeRewards} periodFilter={periodFilter} />
              } />
            </Routes>
          </Box>
        </Box>
      </Router>
    </ThemeProvider>
  );
}

export default App;
