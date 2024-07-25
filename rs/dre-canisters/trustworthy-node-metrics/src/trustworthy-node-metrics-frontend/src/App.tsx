import React, { useState, useEffect } from 'react';
import { Box, CircularProgress, CssBaseline, ThemeProvider, createTheme } from '@mui/material';
import FilterBar, { Filters } from './components/FilterBar';
import Drawer from './components/Drawer'; // Import the updated Drawer component
import { trustworthy_node_metrics } from '../../declarations/trustworthy-node-metrics/index.js'; // Adjust the path as needed
import { SubnetNodeMetricsArgs, SubnetNodeMetricsResult } from '../../declarations/trustworthy-node-metrics/trustworthy-node-metrics.did.js';
import { NodeMetrics } from './models/NodeMetrics';
import ChartGrid from './components/ChartGrid';
import Header from './components/Header';

const darkTheme = createTheme({
  palette: {
    mode: 'dark',
    divider: '#121212',
  },
});

function App() {
  const [filters, setFilters] = useState<Filters>({
    dateStart: new Date(),
    dateEnd: new Date(), 
    subnet: '' 
  });
  const [data, setData] = useState<NodeMetrics[]>([]);
  const [filteredData, setFilteredData] = useState<NodeMetrics[]>([]);
  const [subnets, setSubnets] = useState<Set<string>>(new Set());
  const [error, setError] = useState("");
  const drawerWidth = 120;

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

        return metrics.subnet_id.toText() === filters.subnet && isDateInRange;
      });

      setFilteredData(f);
    }

    filterData();
  }, [filters, data]);

  const handleSubnetSelect = (subnet: string) => {
    setFilters((prev) => ({ ...prev, subnet }));
  };

  if (error) {
    return <div>Error: {error}</div>;
  }

  if (data.length === 0) {
    return <CircularProgress />;
  }

  return (
    <ThemeProvider theme={darkTheme}>
      <CssBaseline />
      <Box sx={{ display: 'flex' }}>
        <Drawer
          subnets={subnets}
          drawerWidth={drawerWidth}
          theme={darkTheme}
          onSubnetSelect={handleSubnetSelect}
        />
        <Box
          component="main"
          sx={{ flexGrow: 1, width: `calc(100% - ${drawerWidth}px)`}}
        >
          <Header />
          <Box sx={{ mb: 2 }}>
            <FilterBar filters={filters} setFilters={setFilters} subnets={subnets} />
          </Box>
          <ChartGrid data={filteredData} />
        </Box>
      </Box>
    </ThemeProvider>
  );
}

export default App;
