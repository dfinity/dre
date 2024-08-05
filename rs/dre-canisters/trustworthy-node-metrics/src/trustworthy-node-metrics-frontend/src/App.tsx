import React, { useState, useEffect, useMemo } from 'react';
import { Box, CircularProgress, CssBaseline, ThemeProvider, createTheme } from '@mui/material';
import FilterBar, { PeriodFilter } from './components/FilterBar';
import Drawer from './components/Drawer'; 
import { BrowserRouter as Router, Route, Routes } from 'react-router-dom';
import { trustworthy_node_metrics } from '../../declarations/trustworthy-node-metrics/index.js';
import { NodeRewardsArgs, NodeRewardsResponse, Rewards, SubnetNodeMetricsArgs, SubnetNodeMetricsResult } from '../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did.js';
import { DailyNodeMetrics, DashboardNodeMetrics, NodeMetrics } from './models/NodeMetrics';
import { NodeList } from './components/NodeList';
import Header from './components/Header';
import { calculateDailyValues, dateToNanoseconds, groupBy } from './utils/utils';
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
  const thirtyDaysAgo = new Date();
  thirtyDaysAgo.setDate(thirtyDaysAgo.getDate() - 30);

  const [periodFilter, setPeriodFilter] = useState<PeriodFilter>({
    dateStart: thirtyDaysAgo,
    dateEnd: new Date()
  });
  const [dailyNodeMetrics, setDailyNodeMetrics] = useState<DailyNodeMetrics[]>([]);
  const [dashboardNodeMetrics, setDashboardNodeMetrics] = useState<DashboardNodeMetrics[]>([]);

  const [subnets, setSubnets] = useState<Set<string>>(new Set());
  const [nodeProviders, setNodeProviders] = useState<Set<string>>(new Set());
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState("");
  const drawerWidth = 180;

  useEffect(() => {
    const fetchNodes = async () => {
      try {
        const request: SubnetNodeMetricsArgs = {
          ts: [],
          subnet_id: [],
        };
        const response: SubnetNodeMetricsResult = await trustworthy_node_metrics.subnet_node_metrics(request);

        if ('Ok' in response) {
          const metrics: NodeMetrics[] = response.Ok.flatMap((metricResponse) => {
            return metricResponse.node_metrics.map((nodeMetrics) => {
              return new NodeMetrics(
                metricResponse.ts,
                nodeMetrics.num_block_failures_total,
                nodeMetrics.node_id,
                nodeMetrics.num_blocks_proposed_total,
                metricResponse.subnet_id
              );
            })
          });

          const grouped = groupBy(metrics, 'nodeId');
          const dailyNodeMetrics = Object.keys(grouped).flatMap(nodeId => {
            const items = grouped[nodeId];
            const dailyData = calculateDailyValues(items);
  
            return dailyData.map(daily => {
              return new DailyNodeMetrics(
                nodeId,
                daily,
              )
            }
          )});

          setDailyNodeMetrics(dailyNodeMetrics);
        } else {
          setError(response.Err);
        }
      } catch (error) {
        console.error("Error fetching nodes:", error);
      } finally {
        setIsLoading(false);
      }
    };
    
    fetchNodes();
  }, []);

  useEffect(() => {
    const updateRewards = async () => {
      try {
        setIsLoading(true)
        const nodeRewardsMap = new Map<string, Rewards>();
        const request: NodeRewardsArgs = {
          from_ts: dateToNanoseconds(periodFilter.dateStart),
          to_ts: dateToNanoseconds(periodFilter.dateEnd),
        };
        const nodeRewardsResponse: NodeRewardsResponse[] = await trustworthy_node_metrics.node_rewards(request);
        nodeRewardsResponse.forEach((nodeReward) => {
          nodeRewardsMap.set(nodeReward.node_id.toText(), nodeReward.node_rewards);
        });
        
        const metricsInPeriod = dailyNodeMetrics.filter((metrics) => {
          const metricsDate = metrics.dailyData.date; 
          const isDateInRange = metricsDate >= periodFilter.dateStart && metricsDate <= periodFilter.dateEnd;
          return isDateInRange;
        });
        const grouped = groupBy(metricsInPeriod, 'nodeId');
        const groupedMetrics = Object.keys(grouped).map(nodeId => {
          const rewards = nodeRewardsMap.get(nodeId);

          if (rewards === undefined) {
            throw new Error('rewards_standard is undefined');
          }

          return new DashboardNodeMetrics(
            nodeId,
            grouped[nodeId].map(data => data.dailyData),
            rewards.rewards_standard
          );
        })
        .sort((a, b) => a.rewardsNoPenalty - b.rewardsNoPenalty);

        const subnets = new Set(metricsInPeriod.map(metric => metric.dailyData.subnetId));

        setDashboardNodeMetrics(groupedMetrics);
        setSubnets(subnets);

      } catch (error) {
        console.error("Error fetching nodes:", error);
      } finally {
        setIsLoading(false);
      }
    };
    
    updateRewards();
  }, [periodFilter, dailyNodeMetrics]);


  if (error) {
    return <div>Error: {error}</div>;
  }

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
              <Route path="/" element={
                isLoading ? (<LoadingIndicator />) : (<NodeList dashboardNodeMetrics={dashboardNodeMetrics} periodFilter={periodFilter} />)} 
                />
              <Route path="/nodes" element={
                isLoading ? (<LoadingIndicator />) : (<NodeList dashboardNodeMetrics={dashboardNodeMetrics} periodFilter={periodFilter} />)} 
                />
              <Route path="/subnets/:subnet" element={
                isLoading ? (<LoadingIndicator />) : (<SubnetChart dashboardNodeMetrics={dashboardNodeMetrics} periodFilter={periodFilter} />)} 
                />
            </Routes>
          </Box>
        </Box>
      </Router>
    </ThemeProvider>
  );
}

export default App;
