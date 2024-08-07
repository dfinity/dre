import React, { useState, useEffect } from 'react';
import { Box, CircularProgress, CssBaseline, ThemeProvider, createTheme } from '@mui/material';
import FilterBar, { PeriodFilter } from './components/FilterBar';
import Drawer from './components/Drawer'; 
import { BrowserRouter as Router, Route, Routes, Navigate } from 'react-router-dom';
import { trustworthy_node_metrics } from '../../declarations/trustworthy-node-metrics/index.js';
import { NodeRewardsArgs, NodeRewardsResponse } from '../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did.js';
import { DashboardNodeRewards } from './models/NodeMetrics';
import { NodeList } from './components/NodeList';
import Header from './components/Header';
import { dateToNanoseconds } from './utils/utils';
import { SubnetChart } from './components/SubnetChart';

const darkTheme = createTheme({
  palette: {
    mode: 'dark',
    divider: '#121212',
  },
});

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

function App() {
  const dateStart = new Date();
  const dateEnd = new Date();
  dateStart.setDate(dateStart.getDate() - 30);
  dateStart.setHours(0, 0, 0, 0);
  dateEnd.setHours(23, 59, 59, 999);

  const [periodFilter, setPeriodFilter] = useState<PeriodFilter>({
    dateStart: dateStart,
    dateEnd: dateEnd
  });

  const [dashboardNodeRewards, setDashboardNodeRewards] = useState<DashboardNodeRewards[]>([]);

  const [subnets, setSubnets] = useState<Set<string>>(new Set());
  const [nodeProviders, setNodeProviders] = useState<Set<string>>(new Set());
  const [isLoading, setIsLoading] = useState(true);
  const drawerWidth = 180;

  useEffect(() => {
    const updateRewards = async () => {
      try {
        setIsLoading(true)
        const request: NodeRewardsArgs = {
          from_ts: dateToNanoseconds(periodFilter.dateStart),
          to_ts: dateToNanoseconds(periodFilter.dateEnd),
        };
        const nodeRewardsResponse: NodeRewardsResponse[] = await trustworthy_node_metrics.node_rewards(request);

        const dashboardNodeRewards = nodeRewardsResponse.map((nodeRewards) => {
          return new DashboardNodeRewards(
            nodeRewards.node_id,
            nodeRewards.daily_metrics,
            nodeRewards.rewards_no_penalty,
            nodeRewards.rewards_with_penalty,
          );
        }).sort((a, b) => a.rewardsNoPenalty - b.rewardsNoPenalty );
        

        const subnets = new Set(dashboardNodeRewards.flatMap(nodeRewards => nodeRewards.dailyData.map( data => data.subnet_assigned.toText())));

        setDashboardNodeRewards(dashboardNodeRewards);
        setSubnets(subnets);

      } catch (error) {
        console.error("Error fetching node:", error);
      } finally {
        setIsLoading(false);
      }
    };
    
    updateRewards();
  }, [periodFilter]);

  return (
    <ThemeProvider theme={darkTheme}>
      <CssBaseline />
      <Router>
        <Box sx={{ display: 'flex' }}>
          <Drawer
            subnets={subnets}
            nodeProviders={nodeProviders}
            drawerWidth={drawerWidth}
          />
          <Box
            component="main"
            sx={{ flexGrow: 1, width: `calc(100% - ${drawerWidth}px)` }}
          >
            <Header />
            <Box sx={{ mb: 2 }}>
              <FilterBar
                filters={periodFilter}
                setFilters={setPeriodFilter}
                subnets={subnets}
              />
            </Box>
            <Routes>
              <Route path="/" element={<Navigate to="/nodes" replace />} />
              <Route path="/nodes" element={
                isLoading ? (<LoadingIndicator />) : (<NodeList dashboardNodeMetrics={dashboardNodeRewards} periodFilter={periodFilter} />)} 
                />
              <Route path="/subnets/:subnet" element={
                isLoading ? (<LoadingIndicator />) : (<SubnetChart dashboardNodeMetrics={dashboardNodeRewards} periodFilter={periodFilter} />)} 
                />
            </Routes>
          </Box>
        </Box>
      </Router>
    </ThemeProvider>
  );
}

export default App;
