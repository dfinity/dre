import React, { useState, useEffect } from 'react';
import { Box, CircularProgress, CssBaseline, ThemeProvider, createTheme } from '@mui/material';
import FilterBar, { Filters } from './components/FilterBar';
import Drawer from './components/Drawer'; 
import { BrowserRouter as Router, Route, Routes, Navigate } from 'react-router-dom';
import { trustworthy_node_metrics } from '../../declarations/trustworthy-node-metrics/index.js'; // Adjust the path as needed
import { SubnetNodeMetricsArgs, SubnetNodeMetricsResult } from '../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did.js';
import { NodeMetrics } from './models/NodeMetrics';
import { ChartGrid } from './components/ChartGrid';
import { StackedChart } from './components/ChartGrid';
import Header from './components/Header';

const darkTheme = createTheme({
  palette: {
    mode: 'dark',
    divider: '#121212',
  },
});

function App() {
  const thirtyDaysAgo = new Date();
  thirtyDaysAgo.setDate(thirtyDaysAgo.getDate() - 30);

  const [filters, setFilters] = useState<Filters>({
    dateStart: thirtyDaysAgo,
    dateEnd: new Date(), 
    subnet: null,
    nodeProvider: null
  });
  const [data, setData] = useState<NodeMetrics[]>([]);
  const [filteredData, setFilteredData] = useState<NodeMetrics[]>([]);
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

        console.info("Got response", response);

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
          const subnets: Set<string> = new Set(metrics.map(metric => metric.subnet_id.toText()));

          setData(metrics);
          setSubnets(subnets);
          setIsLoading(false);
        } else {
          setError(response.Err);
        }
      } catch (error) {
        console.error("Error fetching nodes:", error);
      }
    };
    
    fetchNodes();
  }, []);

  useEffect(() => {
    const filterData = () => {
      const f = data.filter((metrics) => {
        const metricsDate = metrics.date; 
        const isDateInRange = metricsDate >= filters.dateStart && metricsDate <= filters.dateEnd;

        if (filters.subnet !== null) {
          return metrics.subnet_id.toText() === filters.subnet && isDateInRange;
        }

        return isDateInRange;
      });

      setFilteredData(f);
    }
    filterData();
  }, [filters, data]);

  if (error) {
    return <div>Error: {error}</div>;
  }

  if (isLoading) {
    return <CircularProgress />
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
            theme={darkTheme}
            setFilters={setFilters}
          />
          <Box
            component="main"
            sx={{ flexGrow: 1, width: `calc(100% - ${drawerWidth}px)` }}
          >
            <Header />
            <Box sx={{ mb: 2 }}>
              <FilterBar
                filters={filters}
                setFilters={setFilters}
                subnets={subnets}
              />
            </Box>
            <Routes>
              <Route path="/" element={<Navigate to="/nodes" />} />
              <Route path="/nodes" element={<ChartGrid data={data} />} />
              <Route path="/nodes2" element={<StackedChart data={filteredData} name={filters.subnet} />} />
              <Route path="/subnets" element={<StackedChart data={filteredData} name={filters.subnet} />} />
            </Routes>
          </Box>
        </Box>
      </Router>
    </ThemeProvider>
  );
}

export default App;
