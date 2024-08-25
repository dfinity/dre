import React, { useState, useEffect, useMemo } from 'react';
import { Box, CircularProgress, CssBaseline, ThemeProvider, createTheme, useMediaQuery, useTheme } from '@mui/material';
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
  dateStart.setUTCDate(dateStart.getUTCDate() - 30);
  dateStart.setUTCHours(0, 0, 0, 0);
  dateEnd.setUTCHours(23, 59, 59, 999);
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
  const [isLoading, setIsLoading] = useState(true);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const theme = useTheme();
  const isSmallScreen = useMediaQuery(theme.breakpoints.down('sm'));
  const drawerWidth = 180;

  const handleDrawerToggle = () => setDrawerOpen(prev => !prev);

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

  const drawerProps = useMemo(() => ({
    subnets,
    nodeProviders,
    drawerWidth,
    temporary: isSmallScreen,
    drawerOpen,
    onClosed: handleDrawerToggle
  }), [subnets, nodeProviders, drawerWidth, isSmallScreen, drawerOpen]);

  return (
    <ThemeProvider theme={darkTheme}>
      <CssBaseline />
      <Router>
        <Box sx={{ display: 'flex' }}>
          <Drawer {...drawerProps} />
          <Box sx={{ flexGrow: 1, width: `calc(100% - ${drawerWidth}px)` }}>
            <Header withDrawerIcon={isSmallScreen} onDrawerIconClicked={handleDrawerToggle} />
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
